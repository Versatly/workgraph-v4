//! Core shared types for WorkGraph primitives, ledger, and identity.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Built-in primitive kinds available in WorkGraph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveType {
    /// Decision primitive.
    Decision,
    /// Pattern primitive.
    Pattern,
    /// Lesson primitive.
    Lesson,
    /// Policy primitive.
    Policy,
    /// Relationship primitive.
    Relationship,
    /// Strategic note primitive.
    StrategicNote,
    /// Organization primitive.
    Org,
    /// Team primitive.
    Team,
    /// Person primitive.
    Person,
    /// Agent primitive.
    Agent,
    /// Client primitive.
    Client,
    /// Project primitive.
    Project,
}

impl PrimitiveType {
    /// Returns the canonical snake_case name for this primitive type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Pattern => "pattern",
            Self::Lesson => "lesson",
            Self::Policy => "policy",
            Self::Relationship => "relationship",
            Self::StrategicNote => "strategic_note",
            Self::Org => "org",
            Self::Team => "team",
            Self::Person => "person",
            Self::Agent => "agent",
            Self::Client => "client",
            Self::Project => "project",
        }
    }

    /// Returns the on-disk directory name used by this primitive type.
    #[must_use]
    pub const fn directory_name(&self) -> &'static str {
        match self {
            Self::Decision => "decisions",
            Self::Pattern => "patterns",
            Self::Lesson => "lessons",
            Self::Policy => "policies",
            Self::Relationship => "relationships",
            Self::StrategicNote => "strategic_notes",
            Self::Org => "orgs",
            Self::Team => "teams",
            Self::Person => "people",
            Self::Agent => "agents",
            Self::Client => "clients",
            Self::Project => "projects",
        }
    }

    /// Returns all built-in primitive types in deterministic order.
    #[must_use]
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::Decision,
            Self::Pattern,
            Self::Lesson,
            Self::Policy,
            Self::Relationship,
            Self::StrategicNote,
            Self::Org,
            Self::Team,
            Self::Person,
            Self::Agent,
            Self::Client,
            Self::Project,
        ]
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PrimitiveType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "decision" => Ok(Self::Decision),
            "pattern" => Ok(Self::Pattern),
            "lesson" => Ok(Self::Lesson),
            "policy" => Ok(Self::Policy),
            "relationship" => Ok(Self::Relationship),
            "strategic_note" => Ok(Self::StrategicNote),
            "org" => Ok(Self::Org),
            "team" => Ok(Self::Team),
            "person" => Ok(Self::Person),
            "agent" => Ok(Self::Agent),
            "client" => Ok(Self::Client),
            "project" => Ok(Self::Project),
            other => Err(format!("unknown primitive type: {other}")),
        }
    }
}

/// Describes a single field in a primitive schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field name.
    pub name: String,
    /// Human-readable field type description.
    pub field_type: String,
    /// Whether the field is required.
    pub required: bool,
    /// Optional field description.
    pub description: Option<String>,
}

/// Schema metadata for a primitive type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveSchema {
    /// Primitive type this schema describes.
    pub primitive_type: PrimitiveType,
    /// Declared fields for this type.
    pub fields: Vec<FieldDefinition>,
}

/// Registry of primitive schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Registry {
    /// All known schemas keyed by primitive type.
    pub types: BTreeMap<PrimitiveType, PrimitiveSchema>,
}

/// Actor identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub String);

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ActorId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Workspace identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceId(pub String);

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for WorkspaceId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Node identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Workspace configuration persisted under `.workgraph/config.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkgraphConfig {
    /// Stable workspace identifier.
    pub workspace_id: WorkspaceId,
    /// Local node identifier.
    pub node_id: NodeId,
    /// Default actor used by non-interactive commands.
    pub default_actor: ActorId,
    /// Relative store root for primitives.
    pub store_root: String,
    /// Relative ledger path.
    pub ledger_path: String,
}

impl Default for WorkgraphConfig {
    fn default() -> Self {
        Self {
            workspace_id: WorkspaceId("default-workspace".to_owned()),
            node_id: NodeId("local-node".to_owned()),
            default_actor: ActorId("cli".to_owned()),
            store_root: ".".to_owned(),
            ledger_path: ".workgraph/ledger.jsonl".to_owned(),
        }
    }
}

/// External reference to a third-party system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRef {
    /// External provider name.
    pub provider: String,
    /// Reference kind inside the provider.
    pub kind: String,
    /// Absolute URL or provider URI.
    pub url: String,
}

/// Generic cached snapshot metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CachedSnapshot {
    /// Human-friendly title for the snapshot.
    pub title: String,
    /// Optional summary text.
    pub summary: Option<String>,
    /// Linked external references.
    pub external_refs: Vec<ExternalRef>,
    /// Additional metadata fields.
    pub metadata: BTreeMap<String, String>,
}

/// Organization context primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Org {
    /// Primitive ID.
    pub id: String,
    /// Org display name.
    pub title: String,
    /// Optional mission statement.
    pub mission: Option<String>,
    /// External refs.
    pub external_refs: Vec<ExternalRef>,
}

/// Team context primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    /// Primitive ID.
    pub id: String,
    /// Team name.
    pub title: String,
    /// Team responsibilities.
    pub responsibilities: Vec<String>,
    /// External refs.
    pub external_refs: Vec<ExternalRef>,
}

/// Person context primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    /// Primitive ID.
    pub id: String,
    /// Person display name.
    pub title: String,
    /// Preferred communication style.
    pub preferences: Vec<String>,
    /// External refs.
    pub external_refs: Vec<ExternalRef>,
}

/// Agent context primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// Primitive ID.
    pub id: String,
    /// Agent display name.
    pub title: String,
    /// Capability descriptors.
    pub capabilities: Vec<String>,
    /// Connection endpoint or location.
    pub location: Option<String>,
    /// External refs.
    pub external_refs: Vec<ExternalRef>,
}

/// Client context primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    /// Primitive ID.
    pub id: String,
    /// Client name.
    pub title: String,
    /// Current client status.
    pub status: Option<String>,
    /// External refs.
    pub external_refs: Vec<ExternalRef>,
}

/// Project context primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Primitive ID.
    pub id: String,
    /// Project title.
    pub title: String,
    /// Project status.
    pub status: Option<String>,
    /// External refs.
    pub external_refs: Vec<ExternalRef>,
}

/// Decision primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// Primitive ID.
    pub id: String,
    /// Decision title.
    pub title: String,
    /// Decision status.
    pub status: String,
    /// Actor who decided.
    pub decided_by: String,
    /// Date of decision.
    pub decided_at: NaiveDate,
    /// Participants consulted.
    pub participants: Vec<String>,
}

/// Pattern primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    /// Primitive ID.
    pub id: String,
    /// Pattern title.
    pub title: String,
    /// Ordered pattern steps.
    pub steps: Vec<String>,
    /// Optional exceptions.
    pub exceptions: Vec<String>,
}

/// Lesson primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    /// Primitive ID.
    pub id: String,
    /// Lesson title.
    pub title: String,
    /// Source experience.
    pub learned_from: String,
    /// Scope where the lesson applies.
    pub applies_to: Vec<String>,
}

/// Policy primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Primitive ID.
    pub id: String,
    /// Policy title.
    pub title: String,
    /// Enforcement mode.
    pub enforcement: String,
    /// Optional exceptions.
    pub exceptions: Vec<String>,
}

/// Relationship primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    /// Primitive ID.
    pub id: String,
    /// Relationship title.
    pub title: String,
    /// Source entity ID.
    pub source: String,
    /// Target entity ID.
    pub target: String,
    /// Relationship nature.
    pub nature: String,
}

/// Strategic note primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicNote {
    /// Primitive ID.
    pub id: String,
    /// Note title.
    pub title: String,
    /// Optional planning horizon.
    pub horizon: Option<String>,
}

/// Ledger operation kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerOp {
    /// Create operation.
    Create,
    /// Update operation.
    Update,
    /// Delete operation.
    Delete,
}

/// Input payload used before hash materialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntryInput {
    /// Event timestamp.
    pub ts: DateTime<Utc>,
    /// Actor performing the operation.
    pub actor: ActorId,
    /// Operation kind.
    pub op: LedgerOp,
    /// Primitive type affected.
    pub primitive_type: PrimitiveType,
    /// Primitive ID affected.
    pub primitive_id: String,
    /// Changed fields in this mutation.
    pub fields_changed: Vec<String>,
}

/// Immutable ledger entry with hash chain fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Event timestamp.
    pub ts: DateTime<Utc>,
    /// Actor performing the operation.
    pub actor: ActorId,
    /// Operation kind.
    pub op: LedgerOp,
    /// Primitive type affected.
    pub primitive_type: PrimitiveType,
    /// Primitive ID affected.
    pub primitive_id: String,
    /// Changed fields in this mutation.
    pub fields_changed: Vec<String>,
    /// Entry hash.
    pub hash: String,
    /// Previous entry hash.
    pub prev_hash: Option<String>,
}
