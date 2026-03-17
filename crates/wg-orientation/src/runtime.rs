//! Runtime orientation functions backed by store, ledger, and graph data.

use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use serde_yaml::Value;
use wg_error::Result;
use wg_graph::{BrokenLink, build_graph};
use wg_ledger::{LedgerCursor, LedgerReader};
use wg_paths::WorkspacePath;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive, write_primitive};
use wg_types::{ActorId, FieldDefinition, LedgerEntry, PrimitiveType, Registry};

use crate::{BriefItem, RecentActivity};

/// Workspace orientation summary derived from real persisted data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceStatus {
    /// Primitive counts by type.
    pub type_counts: BTreeMap<String, usize>,
    /// Most recent immutable ledger activity.
    pub recent_activity: Vec<RecentActivity>,
    /// Broken wiki-links discovered by the graph engine.
    pub broken_links: Vec<BrokenLink>,
}

/// Actor-specific orientation brief based on assignment and recent activity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActorBrief {
    /// Target actor identifier.
    pub actor: String,
    /// Threads currently assigned to this actor.
    pub assigned_threads: Vec<BriefItem>,
    /// Missions related to assigned threads.
    pub assigned_missions: Vec<BriefItem>,
    /// Recent activity relevant to this actor.
    pub recent_relevant_activity: Vec<RecentActivity>,
    /// Warnings that need attention.
    pub warnings: Vec<String>,
}

/// Builds a workspace status summary from persisted primitives and ledger entries.
///
/// # Errors
///
/// Returns an error when graph, store, or ledger data cannot be loaded.
pub async fn status(workspace: &WorkspacePath) -> Result<WorkspaceStatus> {
    let graph = build_graph(workspace).await?;
    let mut type_counts = BTreeMap::new();

    for node in graph.nodes() {
        *type_counts.entry(node.primitive_type).or_insert(0) += 1;
    }

    Ok(WorkspaceStatus {
        type_counts,
        recent_activity: load_recent_activity(workspace, 10).await?,
        broken_links: graph.broken_links().to_vec(),
    })
}

/// Builds an actor-scoped brief showing assignments and relevant changes.
///
/// # Errors
///
/// Returns an error when store or ledger data cannot be loaded.
pub async fn brief(workspace: &WorkspacePath, actor: &ActorId) -> Result<ActorBrief> {
    let actor_id = actor.as_str();
    let actor_status = status(workspace).await?;
    let thread_primitives = list_primitives(workspace, "thread").await?;
    let assigned_threads = thread_primitives
        .iter()
        .filter(|primitive| {
            primitive
                .frontmatter
                .extra_fields
                .get("assigned_actor")
                .and_then(string_value)
                == Some(actor_id)
        })
        .map(|primitive| brief_item("thread", primitive))
        .collect::<Vec<_>>();
    let assigned_thread_ids = assigned_threads
        .iter()
        .filter_map(|item| item.reference.as_ref())
        .filter_map(|reference| reference.split_once('/').map(|(_, id)| id.to_owned()))
        .collect::<BTreeSet<_>>();

    let mission_primitives = list_primitives(workspace, "mission").await?;
    let assigned_missions = mission_primitives
        .iter()
        .filter(|primitive| {
            parse_string_list(primitive.frontmatter.extra_fields.get("thread_ids"))
                .iter()
                .any(|thread_id| assigned_thread_ids.contains(thread_id))
        })
        .map(|primitive| brief_item("mission", primitive))
        .collect::<Vec<_>>();

    let relevant_refs = assigned_threads
        .iter()
        .chain(&assigned_missions)
        .filter_map(|item| item.reference.clone())
        .collect::<BTreeSet<_>>();
    let ledger_entries = load_ledger_entries(workspace).await?;
    let recent_relevant_activity = ledger_entries
        .into_iter()
        .rev()
        .filter(|entry| {
            entry.actor.as_str() == actor_id
                || relevant_refs.contains(&format!("{}/{}", entry.primitive_type, entry.primitive_id))
        })
        .take(10)
        .map(entry_to_recent_activity)
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    if assigned_threads.is_empty() && assigned_missions.is_empty() {
        warnings.push(format!("Actor '{actor_id}' has no assigned threads or missions"));
    }
    for broken in actor_status
        .broken_links
        .iter()
        .filter(|broken| relevant_refs.contains(&broken.source.reference()))
        .take(5)
    {
        warnings.push(format!(
            "Broken link from '{}' to '{}' ({})",
            broken.source.reference(),
            broken.target,
            broken.reason
        ));
    }

    Ok(ActorBrief {
        actor: actor_id.to_owned(),
        assigned_threads,
        assigned_missions,
        recent_relevant_activity,
        warnings,
    })
}

/// Saves a checkpoint primitive for current work focus.
///
/// # Errors
///
/// Returns an error when checkpoint persistence fails.
pub async fn checkpoint(
    workspace: &WorkspacePath,
    working_on: &str,
    focus: &str,
) -> Result<StoredPrimitive> {
    let id = format!(
        "{}-{}",
        slugify(working_on),
        Utc::now().format("%Y%m%d%H%M%S")
    );
    let title = format!("Checkpoint: {}", working_on.trim());
    let created_at = Utc::now().to_rfc3339();
    let primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: "checkpoint".to_owned(),
            id: id.clone(),
            title,
            extra_fields: BTreeMap::from([
                (
                    "working_on".to_owned(),
                    Value::String(working_on.trim().to_owned()),
                ),
                ("focus".to_owned(), Value::String(focus.trim().to_owned())),
                ("created_at".to_owned(), Value::String(created_at)),
            ]),
        },
        body: format!("## Working on\n{working_on}\n\n## Focus\n{focus}\n"),
    };

    write_primitive(workspace, &checkpoint_registry(), &primitive).await?;
    read_primitive(workspace, "checkpoint", &id).await
}

async fn load_recent_activity(workspace: &WorkspacePath, limit: usize) -> Result<Vec<RecentActivity>> {
    Ok(load_ledger_entries(workspace)
        .await?
        .into_iter()
        .rev()
        .take(limit)
        .map(entry_to_recent_activity)
        .collect())
}

async fn load_ledger_entries(workspace: &WorkspacePath) -> Result<Vec<LedgerEntry>> {
    let reader = LedgerReader::new(workspace.as_path().to_path_buf());
    let (entries, _) = reader.read_from(LedgerCursor::default()).await?;
    Ok(entries)
}

fn entry_to_recent_activity(entry: LedgerEntry) -> RecentActivity {
    RecentActivity {
        ts: entry.ts.to_rfc3339(),
        actor: entry.actor.to_string(),
        op: format!("{:?}", entry.op).to_lowercase(),
        reference: format!("{}/{}", entry.primitive_type, entry.primitive_id),
    }
}

fn brief_item(kind: &str, primitive: &StoredPrimitive) -> BriefItem {
    BriefItem {
        kind: kind.to_owned(),
        reference: Some(format!(
            "{}/{}",
            primitive.frontmatter.r#type, primitive.frontmatter.id
        )),
        title: primitive.frontmatter.title.clone(),
        detail: primitive
            .frontmatter
            .extra_fields
            .get("status")
            .and_then(string_value)
            .map(str::to_owned),
    }
}

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(string_value)
            .map(str::to_owned)
            .collect(),
        Some(Value::Tagged(tagged)) => parse_string_list(Some(&tagged.value)),
        Some(Value::Null | Value::Bool(_) | Value::Number(_) | Value::Mapping(_)) | None => {
            Vec::new()
        }
    }
}

fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => string_value(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Sequence(_) | Value::Mapping(_) => {
            None
        }
    }
}

fn checkpoint_registry() -> Registry {
    let mut registry = Registry::builtins();
    if registry.get_type("checkpoint").is_none() {
        registry.types.push(PrimitiveType::new(
            "checkpoint",
            "checkpoints",
            "Saved orientation checkpoint",
            vec![
                FieldDefinition::new("id", "string", "Stable checkpoint identifier", true, false),
                FieldDefinition::new("title", "string", "Checkpoint title", true, false),
                FieldDefinition::new("working_on", "string", "Current work item", true, false),
                FieldDefinition::new("focus", "string", "Current focus", true, false),
                FieldDefinition::new("created_at", "datetime", "Checkpoint timestamp", true, false),
            ],
        ));
    }
    registry
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in input.chars() {
        let lower = character.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "checkpoint".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{TimeZone, Utc};
    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_clock::MockClock;
    use wg_ledger::{LedgerEntryDraft, LedgerWriter};
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
    use wg_types::{ActorId, LedgerOp, Registry};

    use crate::{brief, checkpoint, status};

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

    #[tokio::test]
    async fn status_reads_real_counts_recent_activity_and_broken_links() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "decision",
                "alpha",
                "Alpha",
                "Linked to [[decision/missing]].",
                [],
            ),
        )
        .await
        .expect("decision should write");

        let clock = MockClock::new(
            Utc.with_ymd_and_hms(2026, 3, 17, 10, 0, 0)
                .single()
                .expect("timestamp should be valid"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock);
        writer
            .append(LedgerEntryDraft::new(
                ActorId::new("pedro"),
                LedgerOp::Create,
                "decision",
                "alpha",
                vec!["title".to_owned()],
            ))
            .await
            .expect("ledger append should succeed");

        let snapshot = status(&workspace).await.expect("status should load");
        assert_eq!(snapshot.type_counts.get("decision"), Some(&1));
        assert_eq!(snapshot.recent_activity.len(), 1);
        assert_eq!(snapshot.recent_activity[0].reference, "decision/alpha");
        assert_eq!(snapshot.broken_links.len(), 1);
    }

    #[tokio::test]
    async fn brief_returns_actor_assignments_and_relevant_changes() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "thread",
                "thread-1",
                "Thread 1",
                "## Conversation\n\n```yaml\n[]\n```\n",
                [
                    ("status", Value::String("claimed".to_owned())),
                    ("assigned_actor", Value::String("pedro".to_owned())),
                ],
            ),
        )
        .await
        .expect("thread should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "mission",
                "mission-1",
                "Mission 1",
                "Objective",
                [(
                    "thread_ids",
                    Value::Sequence(vec![Value::String("thread-1".to_owned())]),
                )],
            ),
        )
        .await
        .expect("mission should write");

        let clock = MockClock::new(
            Utc.with_ymd_and_hms(2026, 3, 17, 10, 0, 0)
                .single()
                .expect("timestamp should be valid"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock.clone());
        writer
            .append(LedgerEntryDraft::new(
                ActorId::new("ana"),
                LedgerOp::Create,
                "thread",
                "thread-1",
                vec!["status".to_owned()],
            ))
            .await
            .expect("first append should succeed");
        clock.set(
            Utc.with_ymd_and_hms(2026, 3, 17, 10, 5, 0)
                .single()
                .expect("timestamp should be valid"),
        );
        writer
            .append(LedgerEntryDraft::new(
                ActorId::new("pedro"),
                LedgerOp::Update,
                "thread",
                "thread-1",
                vec!["body".to_owned()],
            ))
            .await
            .expect("second append should succeed");

        let actor_brief = brief(&workspace, &ActorId::new("pedro"))
            .await
            .expect("brief should load");
        assert_eq!(actor_brief.assigned_threads.len(), 1);
        assert_eq!(
            actor_brief.assigned_threads[0].reference.as_deref(),
            Some("thread/thread-1")
        );
        assert_eq!(actor_brief.assigned_missions.len(), 1);
        assert!(!actor_brief.recent_relevant_activity.is_empty());
    }

    #[tokio::test]
    async fn checkpoint_persists_a_checkpoint_primitive() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let created = checkpoint(&workspace, "Kernel implementation", "Finish policy tests")
            .await
            .expect("checkpoint should write");
        assert_eq!(created.frontmatter.r#type, "checkpoint");
        assert_eq!(
            created
                .frontmatter
                .extra_fields
                .get("working_on")
                .expect("working_on should exist"),
            &Value::String("Kernel implementation".to_owned())
        );
    }
}
