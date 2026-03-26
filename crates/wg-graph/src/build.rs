//! Typed graph builder that scans store primitives for semantic edges.

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value;
use tokio::fs;
use wg_encoding::parse_frontmatter;
use wg_error::Result;
use wg_fs::list_md_files;
use wg_paths::WorkspacePath;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, list_primitives};
use wg_types::{
    EventPattern, EvidenceItem, GraphEdgeKind, GraphEdgeSource, Registry, TriggerActionPlan,
};

use crate::model::{BrokenLink, Edge, GraphSnapshot, NodeRef};

/// Builds a typed graph snapshot from all primitives in a workspace.
///
/// # Errors
///
/// Returns an error when primitive directories cannot be scanned or stored
/// primitives cannot be loaded.
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

    for (source, primitive) in &primitives {
        emit_reference_edges(
            source,
            primitive,
            &nodes,
            &id_index,
            &mut edges,
            &mut broken_links,
        );
        emit_structured_edges(
            source,
            primitive,
            &nodes,
            &id_index,
            &mut edges,
            &mut broken_links,
        );
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

fn emit_reference_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    for link in primitive_links(primitive) {
        resolve_and_record_edge(
            source,
            source,
            &link,
            None,
            GraphEdgeKind::Reference,
            GraphEdgeSource::WikiLink,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
}

fn emit_structured_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    match primitive.frontmatter.r#type.as_str() {
        "agent" => emit_agent_edges(source, primitive, nodes, id_index, edges, broken_links),
        "relationship" => {
            emit_relationship_edges(source, primitive, nodes, id_index, edges, broken_links)
        }
        "thread" => emit_thread_edges(source, primitive, nodes, id_index, edges, broken_links),
        "mission" => emit_mission_edges(source, primitive, nodes, id_index, edges, broken_links),
        "run" => emit_run_edges(source, primitive, nodes, id_index, edges, broken_links),
        "trigger" => emit_trigger_edges(source, primitive, nodes, id_index, edges, broken_links),
        _ => {}
    }
}

fn emit_agent_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    for field_name in ["parent_actor_id", "root_actor_id"] {
        if let Some(actor_reference) = primitive
            .frontmatter
            .extra_fields
            .get(field_name)
            .and_then(string_value)
        {
            resolve_and_record_edge(
                source,
                source,
                actor_reference,
                None,
                GraphEdgeKind::Assignment,
                GraphEdgeSource::Field,
                nodes,
                id_index,
                edges,
                broken_links,
            );
        }
    }
}

fn emit_relationship_edges(
    declared_by: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    let Some(from_value) = primitive
        .frontmatter
        .extra_fields
        .get("from")
        .and_then(string_value)
    else {
        return;
    };
    let Some(to_value) = primitive
        .frontmatter
        .extra_fields
        .get("to")
        .and_then(string_value)
    else {
        return;
    };

    let from = resolve_structured_target(from_value, None, nodes, id_index);
    let to = resolve_structured_target(to_value, None, nodes, id_index);

    match (from, to) {
        (Ok(from), Ok(to)) => {
            edges.insert(Edge {
                source: from,
                target: to,
                kind: GraphEdgeKind::Relationship,
                provenance: GraphEdgeSource::RelationshipPrimitive,
            });
        }
        (Err(reason), _) => {
            broken_links.insert(BrokenLink {
                source: declared_by.clone(),
                target: normalize_structured_target(from_value, None),
                kind: GraphEdgeKind::Relationship,
                provenance: GraphEdgeSource::RelationshipPrimitive,
                reason,
            });
        }
        (_, Err(reason)) => {
            broken_links.insert(BrokenLink {
                source: declared_by.clone(),
                target: normalize_structured_target(to_value, None),
                kind: GraphEdgeKind::Relationship,
                provenance: GraphEdgeSource::RelationshipPrimitive,
                reason,
            });
        }
    }
}

fn emit_thread_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    if let Some(actor) = primitive
        .frontmatter
        .extra_fields
        .get("assigned_actor")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            source,
            actor,
            None,
            GraphEdgeKind::Assignment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    if let Some(parent_mission_id) = primitive
        .frontmatter
        .extra_fields
        .get("parent_mission_id")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            &NodeRef::new("mission", parent_mission_id),
            source.id.as_str(),
            Some("thread"),
            GraphEdgeKind::Containment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    if let Some(value) = primitive.frontmatter.extra_fields.get("evidence") {
        if let Ok(evidence_items) = serde_yaml::from_value::<Vec<EvidenceItem>>(value.clone()) {
            for evidence in evidence_items {
                if let Some(reference) = evidence.reference {
                    resolve_and_record_edge(
                        source,
                        source,
                        &reference,
                        None,
                        GraphEdgeKind::Evidence,
                        GraphEdgeSource::EvidenceRecord,
                        nodes,
                        id_index,
                        edges,
                        broken_links,
                    );
                }
            }
        }
    }
}

fn emit_mission_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    for thread_id in string_list_field(primitive.frontmatter.extra_fields.get("thread_ids")) {
        resolve_and_record_edge(
            source,
            source,
            &thread_id,
            Some("thread"),
            GraphEdgeKind::Containment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    for run_id in string_list_field(primitive.frontmatter.extra_fields.get("run_ids")) {
        resolve_and_record_edge(
            source,
            source,
            &run_id,
            Some("run"),
            GraphEdgeKind::Containment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
}

fn emit_run_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    if let Some(actor_id) = primitive
        .frontmatter
        .extra_fields
        .get("actor_id")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            source,
            actor_id,
            None,
            GraphEdgeKind::Assignment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    if let Some(executor_id) = primitive
        .frontmatter
        .extra_fields
        .get("executor_id")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            source,
            executor_id,
            None,
            GraphEdgeKind::Assignment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    if let Some(thread_id) = primitive
        .frontmatter
        .extra_fields
        .get("thread_id")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            &NodeRef::new("thread", thread_id),
            source.id.as_str(),
            Some("run"),
            GraphEdgeKind::Containment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    if let Some(mission_id) = primitive
        .frontmatter
        .extra_fields
        .get("mission_id")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            &NodeRef::new("mission", mission_id),
            source.id.as_str(),
            Some("run"),
            GraphEdgeKind::Containment,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
    if let Some(parent_run_id) = primitive
        .frontmatter
        .extra_fields
        .get("parent_run_id")
        .and_then(string_value)
    {
        resolve_and_record_edge(
            source,
            source,
            parent_run_id,
            Some("run"),
            GraphEdgeKind::Reference,
            GraphEdgeSource::Field,
            nodes,
            id_index,
            edges,
            broken_links,
        );
    }
}

fn emit_trigger_edges(
    source: &NodeRef,
    primitive: &StoredPrimitive,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    if let Some(value) = primitive.frontmatter.extra_fields.get("action_plans") {
        if let Ok(action_plans) = serde_yaml::from_value::<Vec<TriggerActionPlan>>(value.clone()) {
            for action_plan in action_plans {
                if let Some(target_reference) = action_plan.target_reference {
                    resolve_and_record_edge(
                        source,
                        source,
                        &target_reference,
                        None,
                        GraphEdgeKind::Trigger,
                        GraphEdgeSource::TriggerRule,
                        nodes,
                        id_index,
                        edges,
                        broken_links,
                    );
                }
            }
        }
    }
    if let Some(value) = primitive.frontmatter.extra_fields.get("event_pattern") {
        if let Ok(pattern) = serde_yaml::from_value::<EventPattern>(value.clone()) {
            if pattern.primitive_types.len() == 1 {
                if let Some(primitive_id) = pattern.primitive_id {
                    resolve_and_record_edge(
                        source,
                        source,
                        &primitive_id,
                        Some(pattern.primitive_types[0].as_str()),
                        GraphEdgeKind::Trigger,
                        GraphEdgeSource::TriggerRule,
                        nodes,
                        id_index,
                        edges,
                        broken_links,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_and_record_edge(
    declared_by: &NodeRef,
    edge_source: &NodeRef,
    raw_target: &str,
    expected_type: Option<&str>,
    kind: GraphEdgeKind,
    provenance: GraphEdgeSource,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
    edges: &mut BTreeSet<Edge>,
    broken_links: &mut BTreeSet<BrokenLink>,
) {
    match resolve_structured_target(raw_target, expected_type, nodes, id_index) {
        Ok(target) => {
            edges.insert(Edge {
                source: edge_source.clone(),
                target,
                kind,
                provenance,
            });
        }
        Err(reason) => {
            broken_links.insert(BrokenLink {
                source: declared_by.clone(),
                target: normalize_structured_target(raw_target, expected_type),
                kind,
                provenance,
                reason,
            });
        }
    }
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

fn normalize_structured_target(raw_target: &str, expected_type: Option<&str>) -> String {
    match expected_type {
        Some(expected_type) if !raw_target.contains('/') => format!("{expected_type}/{raw_target}"),
        _ => raw_target.to_owned(),
    }
}

fn index_nodes_by_id(nodes: &BTreeSet<NodeRef>) -> BTreeMap<String, Vec<NodeRef>> {
    let mut index: BTreeMap<String, Vec<NodeRef>> = BTreeMap::new();
    for node in nodes {
        index.entry(node.id.clone()).or_default().push(node.clone());
    }
    index
}

fn resolve_structured_target(
    raw_target: &str,
    expected_type: Option<&str>,
    nodes: &BTreeSet<NodeRef>,
    id_index: &BTreeMap<String, Vec<NodeRef>>,
) -> std::result::Result<NodeRef, String> {
    let normalized = normalize_structured_target(raw_target, expected_type);
    resolve_target(&normalized, nodes, id_index)
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

fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => string_value(&tagged.value),
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Sequence(_)
        | Value::Mapping(_) => None,
    }
}

fn string_list_field(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(string_value)
            .map(str::to_owned)
            .collect(),
        Some(Value::Tagged(tagged)) => string_list_field(Some(&tagged.value)),
        Some(Value::Null | Value::Bool(_) | Value::Number(_) | Value::Mapping(_)) | None => {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
    use wg_types::{GraphEdgeKind, GraphEdgeSource, Registry, ThreadStatus};

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
    async fn graph_extracts_wiki_reference_edges() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(
            &workspace,
            "decision",
            "alpha",
            "Alpha",
            "Depends on [[decision/beta]].",
            [],
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

        let graph = build_graph(&workspace).await.expect("graph should build");
        assert!(graph.edges().iter().any(|edge| {
            edge.source == NodeRef::new("decision", "alpha")
                && edge.target == NodeRef::new("decision", "beta")
                && edge.kind == GraphEdgeKind::Reference
                && edge.provenance == GraphEdgeSource::WikiLink
        }));
    }

    #[tokio::test]
    async fn graph_emits_assignment_and_containment_edges() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(&workspace, "agent", "cursor", "Cursor", "Profile", []).await;
        write(
            &workspace,
            "mission",
            "mission-1",
            "Mission 1",
            "Objective",
            [(
                "thread_ids",
                Value::Sequence(vec![Value::String("thread-1".to_owned())]),
            )],
        )
        .await;
        write(
            &workspace,
            "thread",
            "thread-1",
            "Thread 1",
            "## Conversation\n\n```yaml\n[]\n```\n",
            [
                (
                    "status",
                    serde_yaml::to_value(ThreadStatus::Active).expect("status should serialize"),
                ),
                ("assigned_actor", Value::String("cursor".to_owned())),
                ("parent_mission_id", Value::String("mission-1".to_owned())),
            ],
        )
        .await;
        write(
            &workspace,
            "run",
            "run-1",
            "Run 1",
            "Summary",
            [
                ("actor_id", Value::String("cursor".to_owned())),
                ("thread_id", Value::String("thread-1".to_owned())),
                ("mission_id", Value::String("mission-1".to_owned())),
            ],
        )
        .await;

        let graph = build_graph(&workspace).await.expect("graph should build");
        let edges = graph.edges();
        assert!(edges.iter().any(|edge| {
            edge.source == NodeRef::new("thread", "thread-1")
                && edge.target == NodeRef::new("agent", "cursor")
                && edge.kind == GraphEdgeKind::Assignment
        }));
        assert!(edges.iter().any(|edge| {
            edge.source == NodeRef::new("mission", "mission-1")
                && edge.target == NodeRef::new("thread", "thread-1")
                && edge.kind == GraphEdgeKind::Containment
        }));
        assert!(edges.iter().any(|edge| {
            edge.source == NodeRef::new("thread", "thread-1")
                && edge.target == NodeRef::new("run", "run-1")
                && edge.kind == GraphEdgeKind::Containment
        }));
    }

    #[tokio::test]
    async fn graph_emits_relationship_and_evidence_edges() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(&workspace, "person", "pedro", "Pedro", "Profile", []).await;
        write(&workspace, "client", "acme", "Acme", "Client", []).await;
        write(
            &workspace,
            "relationship",
            "pedro-owns-acme",
            "Pedro owns Acme",
            "Relationship body",
            [
                ("from", Value::String("pedro".to_owned())),
                ("to", Value::String("acme".to_owned())),
            ],
        )
        .await;
        write(&workspace, "decision", "proof", "Proof", "Decision", []).await;
        write(
            &workspace,
            "thread",
            "thread-1",
            "Thread 1",
            "## Conversation\n\n```yaml\n[]\n```\n",
            [(
                "evidence",
                serde_yaml::to_value(vec![BTreeMap::from([
                    ("id".to_owned(), Value::String("e-1".to_owned())),
                    ("title".to_owned(), Value::String("Proof".to_owned())),
                    ("reference".to_owned(), Value::String("proof".to_owned())),
                    (
                        "satisfies".to_owned(),
                        Value::Sequence(vec![Value::String("criterion-1".to_owned())]),
                    ),
                ])])
                .expect("evidence should serialize"),
            )],
        )
        .await;

        let graph = build_graph(&workspace).await.expect("graph should build");
        let edges = graph.edges();
        assert!(edges.iter().any(|edge| {
            edge.source == NodeRef::new("person", "pedro")
                && edge.target == NodeRef::new("client", "acme")
                && edge.kind == GraphEdgeKind::Relationship
        }));
        assert!(edges.iter().any(|edge| {
            edge.source == NodeRef::new("thread", "thread-1")
                && edge.target == NodeRef::new("decision", "proof")
                && edge.kind == GraphEdgeKind::Evidence
        }));
    }

    #[tokio::test]
    async fn graph_reports_broken_structured_references() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(
            &workspace,
            "thread",
            "thread-1",
            "Thread 1",
            "## Conversation\n\n```yaml\n[]\n```\n",
            [("assigned_actor", Value::String("missing-actor".to_owned()))],
        )
        .await;

        let graph = build_graph(&workspace).await.expect("graph should build");
        assert_eq!(graph.broken_links().len(), 1);
        assert_eq!(graph.broken_links()[0].kind, GraphEdgeKind::Assignment);
        assert_eq!(graph.broken_links()[0].provenance, GraphEdgeSource::Field);
    }

    #[tokio::test]
    async fn graph_reports_orphans_reachable_nodes_and_neighbors() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write(
            &workspace,
            "decision",
            "alpha",
            "Alpha",
            "Depends on [[beta]].",
            [],
        )
        .await;
        write(
            &workspace,
            "decision",
            "beta",
            "Beta",
            "Depends on [[gamma]].",
            [],
        )
        .await;
        write(
            &workspace,
            "decision",
            "gamma",
            "Gamma",
            "No outbound links.",
            [],
        )
        .await;

        let graph = build_graph(&workspace).await.expect("graph should build");
        assert_eq!(
            graph.neighbors(
                &NodeRef::new("decision", "beta"),
                NeighborDirection::Inbound
            ),
            vec![NodeRef::new("decision", "alpha")]
        );
        assert_eq!(
            graph.reachable(&NodeRef::new("decision", "alpha")),
            vec![
                NodeRef::new("decision", "beta"),
                NodeRef::new("decision", "gamma")
            ]
        );
        assert_eq!(
            graph.orphans().into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([NodeRef::new("decision", "alpha")])
        );
    }
}
