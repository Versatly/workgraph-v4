//! Graph builder that scans store primitives for wiki-links.

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value;
use tokio::fs;
use wg_encoding::parse_frontmatter;
use wg_error::Result;
use wg_fs::list_md_files;
use wg_paths::WorkspacePath;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, list_primitives};
use wg_types::Registry;

use crate::model::{BrokenLink, Edge, GraphSnapshot, NodeRef};

/// Builds a directed wiki-link graph snapshot from all primitives in a workspace.
///
/// The builder scans primitive markdown bodies and YAML frontmatter field values
/// for `[[wiki-links]]`. Links may be explicit `[[type/id]]` references or
/// ID-only links such as `[[my-decision]]`.
///
/// # Errors
///
/// Returns an error when primitive directories cannot be scanned or a primitive
/// cannot be loaded from the store.
pub async fn build_graph(workspace: &WorkspacePath) -> Result<GraphSnapshot> {
    let primitive_types = discover_primitive_types(workspace).await?;
    let mut primitives = BTreeMap::new();

    for primitive_type in primitive_types {
        for primitive in list_primitives(workspace, &primitive_type).await? {
            let node = NodeRef::new(
                primitive.frontmatter.r#type.as_str(),
                primitive.frontmatter.id.as_str(),
            );
            primitives.insert(node, primitive);
        }
    }

    let nodes = primitives.keys().cloned().collect::<BTreeSet<_>>();
    let id_index = index_nodes_by_id(&nodes);
    let mut edges = BTreeSet::new();
    let mut broken_links = BTreeSet::new();

    for (source, primitive) in primitives {
        for link in primitive_links(&primitive) {
            match resolve_target(&link, &nodes, &id_index) {
                Ok(target) => {
                    edges.insert(Edge {
                        source: source.clone(),
                        target,
                    });
                }
                Err(reason) => {
                    broken_links.insert(BrokenLink {
                        source: source.clone(),
                        target: link,
                        reason,
                    });
                }
            }
        }
    }

    Ok(GraphSnapshot::from_parts(
        nodes,
        edges,
        broken_links.into_iter().collect(),
    ))
}

async fn discover_primitive_types(workspace: &WorkspacePath) -> Result<BTreeSet<String>> {
    let mut types = Registry::builtins()
        .list_types()
        .iter()
        .map(|primitive_type| primitive_type.name.clone())
        .collect::<BTreeSet<_>>();
    let mut entries = fs::read_dir(workspace.as_path()).await?;

    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }

        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        let markdown_files = match list_md_files(entry.path()).await {
            Ok(paths) => paths,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for path in markdown_files {
            let document = fs::read_to_string(path).await?;
            if let Ok(parsed) = parse_frontmatter::<PrimitiveFrontmatter>(&document) {
                types.insert(parsed.frontmatter.r#type);
            }
        }
    }

    Ok(types)
}

fn primitive_links(primitive: &StoredPrimitive) -> Vec<String> {
    let mut links = extract_wiki_links(&primitive.body);
    for value in primitive.frontmatter.extra_fields.values() {
        extract_links_from_yaml(value, &mut links);
    }
    links
}

fn extract_links_from_yaml(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.extend(extract_wiki_links(text)),
        Value::Sequence(items) => {
            for item in items {
                extract_links_from_yaml(item, out);
            }
        }
        Value::Mapping(entries) => {
            for (key, value) in entries {
                extract_links_from_yaml(key, out);
                extract_links_from_yaml(value, out);
            }
        }
        Value::Tagged(tagged) => extract_links_from_yaml(&tagged.value, out),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn extract_wiki_links(input: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut cursor = 0;

    while let Some(open_offset) = input[cursor..].find("[[") {
        let start = cursor + open_offset + 2;
        let Some(close_offset) = input[start..].find("]]") else {
            break;
        };

        let end = start + close_offset;
        let raw_target = &input[start..end];
        if let Some(target) = normalize_link_target(raw_target) {
            links.push(target);
        }

        cursor = end + 2;
    }

    links
}

fn normalize_link_target(raw_target: &str) -> Option<String> {
    let without_alias = raw_target.split('|').next()?.trim();
    let without_anchor = without_alias.split('#').next()?.trim();
    let without_suffix = without_anchor.strip_suffix(".md").unwrap_or(without_anchor);

    if without_suffix.is_empty() {
        None
    } else {
        Some(without_suffix.to_owned())
    }
}

fn index_nodes_by_id(nodes: &BTreeSet<NodeRef>) -> BTreeMap<String, Vec<NodeRef>> {
    let mut index: BTreeMap<String, Vec<NodeRef>> = BTreeMap::new();
    for node in nodes {
        index.entry(node.id.clone()).or_default().push(node.clone());
    }
    index
}

fn resolve_target(
    link_target: &str,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
) -> std::result::Result<NodeRef, String> {
    if let Some(explicit_target) = NodeRef::from_reference(link_target) {
        if nodes.contains(&explicit_target) {
            return Ok(explicit_target);
        }
        return Err("target primitive does not exist".to_owned());
    }

    match id_index.get(link_target).map(Vec::as_slice) {
        Some([resolved]) => Ok(resolved.clone()),
        Some([]) | None => Err("target primitive does not exist".to_owned()),
        Some(_) => Err("target identifier is ambiguous across multiple primitive types".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
    use wg_types::Registry;

    use super::build_graph;
    use crate::{NeighborDirection, NodeRef};

    fn primitive(
        primitive_type: &str,
        id: &str,
        title: &str,
        body: &str,
        extra_fields: impl IntoIterator<Item = (&'static str, Value)>,
    ) -> StoredPrimitive {
        let mut fields = BTreeMap::new();
        for (key, value) in extra_fields {
            fields.insert(key.to_owned(), value);
        }
        StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: primitive_type.to_owned(),
                id: id.to_owned(),
                title: title.to_owned(),
                extra_fields: fields,
            },
            body: body.to_owned(),
        }
    }

    async fn write(
        workspace: &WorkspacePath,
        primitive_type: &str,
        id: &str,
        title: &str,
        body: &str,
        extra_fields: impl IntoIterator<Item = (&'static str, Value)>,
    ) {
        write_primitive(
            workspace,
            &Registry::builtins(),
            &primitive(primitive_type, id, title, body, extra_fields),
        )
        .await
        .expect("primitive should be written");
    }

    #[tokio::test]
    async fn graph_extracts_links_from_markdown_body_and_yaml_fields() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(
            &workspace,
            "decision",
            "alpha",
            "Alpha",
            "Depends on [[decision/beta]] and [[gamma]].",
            [("summary", Value::String("Related: [[project/gamma]]".to_owned()))],
        )
        .await;
        write(
            &workspace,
            "decision",
            "beta",
            "Beta",
            "No outbound links.",
            [],
        )
        .await;
        write(
            &workspace,
            "project",
            "gamma",
            "Gamma",
            "No outbound links.",
            [],
        )
        .await;

        let graph = build_graph(&workspace)
            .await
            .expect("graph should build successfully");
        let alpha = NodeRef::new("decision", "alpha");

        let outbound = graph.neighbors(&alpha, NeighborDirection::Outbound);
        assert_eq!(
            outbound,
            vec![
                NodeRef::new("decision", "beta"),
                NodeRef::new("project", "gamma")
            ]
        );
        assert!(graph.broken_links().is_empty());
    }

    #[tokio::test]
    async fn graph_reports_orphans_reachable_nodes_and_broken_links() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(
            &workspace,
            "decision",
            "a",
            "A",
            "Links to [[decision/b]] and [[decision/missing]].",
            [],
        )
        .await;
        write(
            &workspace,
            "decision",
            "b",
            "B",
            "No outbound links.",
            [],
        )
        .await;
        write(
            &workspace,
            "decision",
            "c",
            "C",
            "No outbound links.",
            [],
        )
        .await;

        let graph = build_graph(&workspace)
            .await
            .expect("graph should build successfully");
        let a = NodeRef::new("decision", "a");

        assert_eq!(graph.reachable(&a), vec![NodeRef::new("decision", "b")]);
        assert_eq!(
            graph.orphans(),
            vec![NodeRef::new("decision", "a"), NodeRef::new("decision", "c")]
        );
        assert_eq!(graph.broken_links().len(), 1);
        assert_eq!(graph.broken_links()[0].source, a);
        assert_eq!(graph.broken_links()[0].target, "decision/missing");
    }

    #[tokio::test]
    async fn id_only_links_are_rejected_when_identifier_is_ambiguous() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(
            &workspace,
            "decision",
            "shared",
            "Decision Shared",
            "No outbound links.",
            [],
        )
        .await;
        write(
            &workspace,
            "project",
            "shared",
            "Project Shared",
            "No outbound links.",
            [],
        )
        .await;
        write(
            &workspace,
            "decision",
            "source",
            "Source",
            "Mentions [[shared]].",
            [],
        )
        .await;

        let graph = build_graph(&workspace)
            .await
            .expect("graph should build successfully");

        assert_eq!(graph.edges().len(), 0);
        assert_eq!(graph.broken_links().len(), 1);
        assert!(
            graph.broken_links()[0]
                .reason
                .contains("ambiguous across multiple primitive types")
        );
    }

    #[test]
    fn normalize_link_target_strips_aliases_and_anchors() {
        assert_eq!(
            super::normalize_link_target(" decision/alpha | label "),
            Some("decision/alpha".to_owned())
        );
        assert_eq!(
            super::normalize_link_target("project/roadmap#milestone-1"),
            Some("project/roadmap".to_owned())
        );
        assert_eq!(super::normalize_link_target(""), None);
    }

    #[test]
    fn resolve_target_rejects_invalid_type_id_references() {
        let nodes = [NodeRef::new("decision", "alpha")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let index = super::index_nodes_by_id(&nodes);

        let error = super::resolve_target("decision/", &nodes, &index)
            .expect_err("malformed link should fail");
        assert!(error.contains("does not exist"));
    }

    #[test]
    fn extract_wiki_links_collects_multiple_targets() {
        let links = super::extract_wiki_links(
            "Start [[decision/a]] middle [[b|label]] tail [[project/c#anchor]].",
        );
        assert_eq!(links, vec!["decision/a", "b", "project/c"]);
    }

    #[test]
    fn yaml_link_extraction_walks_nested_fields() {
        let nested = Value::Sequence(vec![Value::Mapping(
            [(
                Value::String("note".to_owned()),
                Value::String("See [[decision/alpha]]".to_owned()),
            )]
            .into_iter()
            .collect(),
        )]);
        let mut links = Vec::new();

        super::extract_links_from_yaml(&nested, &mut links);

        assert_eq!(links, vec!["decision/alpha"]);
    }

    #[test]
    fn resolve_target_prefers_explicit_references() {
        let nodes = [
            NodeRef::new("decision", "alpha"),
            NodeRef::new("project", "alpha"),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let index = super::index_nodes_by_id(&nodes);

        let resolved = super::resolve_target("project/alpha", &nodes, &index)
            .expect("explicit type reference should resolve");
        assert_eq!(resolved, NodeRef::new("project", "alpha"));
    }

    #[test]
    fn resolve_target_uses_unique_identifier_lookup() {
        let nodes = [NodeRef::new("decision", "alpha")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let index = super::index_nodes_by_id(&nodes);

        let resolved = super::resolve_target("alpha", &nodes, &index)
            .expect("unique id should resolve");
        assert_eq!(resolved, NodeRef::new("decision", "alpha"));
    }

    #[test]
    fn resolve_target_reports_missing_targets() {
        let nodes = [NodeRef::new("decision", "alpha")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let index = super::index_nodes_by_id(&nodes);

        let error =
            super::resolve_target("missing", &nodes, &index).expect_err("missing id should fail");
        assert!(error.contains("does not exist"));
    }

}
