//! Serializable primitive type definitions and built-in registry state.

use serde::{Deserialize, Serialize};

/// Describes a field available on a primitive type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// The stable field name used in serialized payloads and frontmatter.
    pub name: String,
    /// The logical field type, such as `string`, `datetime`, or `string[]`.
    pub field_type: String,
    /// A short explanation of the field's intent.
    pub description: String,
    /// Whether the field must be present when the primitive is created.
    pub required: bool,
    /// Whether the field can hold multiple values.
    pub repeated: bool,
}

impl FieldDefinition {
    /// Creates a new field definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        field_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
        repeated: bool,
    ) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            description: description.into(),
            required,
            repeated,
        }
    }
}

/// Defines a primitive type and the directory used to store it on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveType {
    /// The singular stable type name, such as `decision` or `project`.
    pub name: String,
    /// The explicit plural directory name used for markdown storage.
    pub directory: String,
    /// A short human-readable description of the primitive.
    pub description: String,
    /// The field definitions supported by this primitive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDefinition>,
}

impl PrimitiveType {
    /// Creates a new primitive type definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        directory: impl Into<String>,
        description: impl Into<String>,
        fields: Vec<FieldDefinition>,
    ) -> Self {
        Self {
            name: name.into(),
            directory: directory.into(),
            description: description.into(),
            fields,
        }
    }

    /// Returns the definition for a named field, if present.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields
            .iter()
            .find(|definition| definition.name == name)
    }
}

/// Serializable registry state for primitive type definitions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// The complete ordered set of known primitive types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<PrimitiveType>,
}

impl Registry {
    /// Creates a new registry from serialized primitive type definitions.
    #[must_use]
    pub fn new(types: Vec<PrimitiveType>) -> Self {
        Self { types }
    }

    /// Constructs a registry populated with the built-in WorkGraph primitive types.
    #[must_use]
    pub fn builtins() -> Self {
        Self {
            types: builtin_types(),
        }
    }

    /// Looks up a primitive type by its stable name.
    #[must_use]
    pub fn get_type(&self, name: &str) -> Option<&PrimitiveType> {
        self.types
            .iter()
            .find(|primitive_type| primitive_type.name == name)
    }

    /// Returns all registered type definitions in their stored order.
    #[must_use]
    pub fn list_types(&self) -> &[PrimitiveType] {
        &self.types
    }
}

fn builtin_types() -> Vec<PrimitiveType> {
    vec![
        builtin_type(
            "org",
            "orgs",
            "Top-level company context and identity.",
            vec![
                field(
                    "id",
                    "string",
                    "Stable organization identifier",
                    true,
                    false,
                ),
                field("title", "string", "Organization display name", true, false),
                field("summary", "string", "Brief operating context", false, false),
            ],
        ),
        builtin_type(
            "team",
            "teams",
            "Operational group of people and agents.",
            vec![
                field("id", "string", "Stable team identifier", true, false),
                field("title", "string", "Team display name", true, false),
                field(
                    "org_id",
                    "string",
                    "Owning organization identifier",
                    false,
                    false,
                ),
                field(
                    "members",
                    "string[]",
                    "Human and agent members",
                    false,
                    true,
                ),
            ],
        ),
        builtin_type(
            "person",
            "people",
            "Human collaborator profile.",
            vec![
                field("id", "string", "Stable person identifier", true, false),
                field("title", "string", "Display name", true, false),
                field("email", "string", "Preferred email address", false, false),
                field("team_ids", "string[]", "Associated teams", false, true),
            ],
        ),
        builtin_type(
            "agent",
            "agents",
            "AI agent profile and runtime capabilities.",
            vec![
                field("id", "string", "Stable agent identifier", true, false),
                field("title", "string", "Agent display name", true, false),
                field("runtime", "string", "Execution environment", false, false),
                field(
                    "parent_actor_id",
                    "string",
                    "Optional tracked parent actor",
                    false,
                    false,
                ),
                field(
                    "root_actor_id",
                    "string",
                    "Optional root tracked actor",
                    false,
                    false,
                ),
                field(
                    "lineage_mode",
                    "string",
                    "Whether descendants are tracked or opaque",
                    false,
                    false,
                ),
                field(
                    "capabilities",
                    "string[]",
                    "Advertised capabilities",
                    false,
                    true,
                ),
            ],
        ),
        builtin_type(
            "client",
            "clients",
            "Customer account context.",
            vec![
                field("id", "string", "Stable client identifier", true, false),
                field("title", "string", "Client display name", true, false),
                field("account_owner", "string", "Primary owner", false, false),
                field(
                    "summary",
                    "string",
                    "Customer context summary",
                    false,
                    false,
                ),
            ],
        ),
        builtin_type(
            "project",
            "projects",
            "Work container linked to external delivery systems.",
            vec![
                field("id", "string", "Stable project identifier", true, false),
                field("title", "string", "Project display name", true, false),
                field("status", "string", "Current project status", false, false),
                field("client_id", "string", "Associated client", false, false),
            ],
        ),
        builtin_type(
            "decision",
            "decisions",
            "Captured rationale and outcomes for important choices.",
            vec![
                field("id", "string", "Stable decision identifier", true, false),
                field("title", "string", "Decision title", true, false),
                field("status", "string", "Decision status", false, false),
                field(
                    "decided_by",
                    "actor_id",
                    "Primary decision maker",
                    false,
                    false,
                ),
            ],
        ),
        builtin_type(
            "pattern",
            "patterns",
            "Repeatable process or operating playbook.",
            vec![
                field("id", "string", "Stable pattern identifier", true, false),
                field("title", "string", "Pattern title", true, false),
                field("steps", "string[]", "Pattern steps", false, true),
            ],
        ),
        builtin_type(
            "lesson",
            "lessons",
            "Knowledge gained from prior work or incidents.",
            vec![
                field("id", "string", "Stable lesson identifier", true, false),
                field("title", "string", "Lesson title", true, false),
                field(
                    "applies_to",
                    "string[]",
                    "Contexts where the lesson applies",
                    false,
                    true,
                ),
            ],
        ),
        builtin_type(
            "policy",
            "policies",
            "Rules, scopes, and enforcement expectations.",
            vec![
                field("id", "string", "Stable policy identifier", true, false),
                field("title", "string", "Policy title", true, false),
                field("scope", "string", "Scope of the policy", false, false),
                field("rule", "string", "Normative rule statement", false, false),
            ],
        ),
        builtin_type(
            "relationship",
            "relationships",
            "Context-rich relationship between two entities.",
            vec![
                field(
                    "id",
                    "string",
                    "Stable relationship identifier",
                    true,
                    false,
                ),
                field("title", "string", "Relationship title", true, false),
                field("from", "string", "Origin entity identifier", true, false),
                field("to", "string", "Target entity identifier", true, false),
            ],
        ),
        builtin_type(
            "strategic_note",
            "strategic_notes",
            "Long-term context about company direction or market understanding.",
            vec![
                field("id", "string", "Stable note identifier", true, false),
                field("title", "string", "Strategic note title", true, false),
                field("horizon", "string", "Planning horizon", false, false),
                field(
                    "body",
                    "string",
                    "Long-form strategic narrative",
                    false,
                    false,
                ),
            ],
        ),
        builtin_type(
            "thread",
            "threads",
            "Evidence-bearing coordination thread.",
            vec![
                field("id", "string", "Stable thread identifier", true, false),
                field("title", "string", "Thread title", true, false),
                field(
                    "status",
                    "thread_status",
                    "Thread lifecycle status",
                    false,
                    false,
                ),
                field(
                    "assigned_actor",
                    "string",
                    "Assigned actor for the thread",
                    false,
                    false,
                ),
                field(
                    "parent_mission_id",
                    "string",
                    "Parent mission identifier",
                    false,
                    false,
                ),
                field(
                    "exit_criteria",
                    "object[]",
                    "Structured exit criteria for completion",
                    false,
                    true,
                ),
                field(
                    "evidence",
                    "object[]",
                    "Evidence recorded against the thread",
                    false,
                    true,
                ),
                field(
                    "update_actions",
                    "object[]",
                    "Planned actions while the thread remains active",
                    false,
                    true,
                ),
                field(
                    "completion_actions",
                    "object[]",
                    "Planned actions once the thread completes",
                    false,
                    true,
                ),
            ],
        ),
        builtin_type(
            "run",
            "runs",
            "Execution instance for an agent or workflow.",
            vec![
                field("id", "string", "Stable run identifier", true, false),
                field("title", "string", "Run title", true, false),
                field("status", "run_status", "Run lifecycle status", false, false),
                field(
                    "actor_id",
                    "string",
                    "Logical actor responsible for the run",
                    true,
                    false,
                ),
                field(
                    "executor_id",
                    "string",
                    "Concrete executor that performed the run",
                    false,
                    false,
                ),
                field(
                    "thread_id",
                    "string",
                    "Owning thread identifier",
                    true,
                    false,
                ),
                field(
                    "mission_id",
                    "string",
                    "Related mission identifier",
                    false,
                    false,
                ),
                field(
                    "parent_run_id",
                    "string",
                    "Parent delegated run identifier",
                    false,
                    false,
                ),
                field(
                    "started_at",
                    "datetime",
                    "Timestamp when run execution started",
                    false,
                    false,
                ),
                field(
                    "ended_at",
                    "datetime",
                    "Timestamp when run execution ended",
                    false,
                    false,
                ),
            ],
        ),
        builtin_type(
            "mission",
            "missions",
            "Coordinated multi-run objective definition.",
            vec![
                field("id", "string", "Stable mission identifier", true, false),
                field("title", "string", "Mission title", true, false),
                field(
                    "status",
                    "mission_status",
                    "Mission lifecycle status",
                    false,
                    false,
                ),
                field(
                    "thread_ids",
                    "string[]",
                    "Child thread identifiers",
                    false,
                    true,
                ),
                field("run_ids", "string[]", "Child run identifiers", false, true),
                field(
                    "milestones",
                    "object[]",
                    "Planned mission milestones with auto-created thread ids",
                    false,
                    true,
                ),
                field(
                    "approved_at",
                    "datetime",
                    "Timestamp when mission approval was recorded",
                    false,
                    false,
                ),
                field(
                    "started_at",
                    "datetime",
                    "Timestamp when mission execution started",
                    false,
                    false,
                ),
                field(
                    "validated_at",
                    "datetime",
                    "Timestamp when mission validation started",
                    false,
                    false,
                ),
                field(
                    "completed_at",
                    "datetime",
                    "Timestamp when mission completion was recorded",
                    false,
                    false,
                ),
                field(
                    "body",
                    "string",
                    "Mission objective markdown body",
                    true,
                    false,
                ),
            ],
        ),
        builtin_type(
            "trigger",
            "triggers",
            "Reactive automation definition driven by ledger events.",
            vec![
                field("id", "string", "Stable trigger identifier", true, false),
                field("title", "string", "Trigger title", true, false),
                field(
                    "status",
                    "trigger_status",
                    "Trigger lifecycle status",
                    false,
                    false,
                ),
                field(
                    "event_pattern",
                    "object",
                    "Structured trigger event matching contract",
                    true,
                    false,
                ),
                field(
                    "action_plans",
                    "object[]",
                    "Durable action plans emitted by the trigger",
                    false,
                    true,
                ),
            ],
        ),
        builtin_type(
            "checkpoint",
            "checkpoints",
            "Durable saved working context.",
            vec![
                field("id", "string", "Stable checkpoint identifier", true, false),
                field("title", "string", "Checkpoint title", true, false),
                field("working_on", "string", "Current work item", true, false),
                field("focus", "string", "Current focus", true, false),
                field(
                    "actor_id",
                    "string",
                    "Owning actor identifier",
                    false,
                    false,
                ),
                field(
                    "created_at",
                    "datetime",
                    "Checkpoint creation timestamp",
                    true,
                    false,
                ),
            ],
        ),
    ]
}

fn builtin_type(
    name: &str,
    directory: &str,
    description: &str,
    fields: Vec<FieldDefinition>,
) -> PrimitiveType {
    PrimitiveType::new(name, directory, description, fields)
}

fn field(
    name: &str,
    field_type: &str,
    description: &str,
    required: bool,
    repeated: bool,
) -> FieldDefinition {
    FieldDefinition::new(name, field_type, description, required, repeated)
}

#[cfg(test)]
mod tests {
    use super::{FieldDefinition, PrimitiveType, Registry};

    #[test]
    fn primitive_type_roundtrips_through_json() {
        let primitive_type = PrimitiveType::new(
            "decision",
            "decisions",
            "Captures important rationale.",
            vec![
                FieldDefinition::new("id", "string", "Stable identifier", true, false),
                FieldDefinition::new("title", "string", "Human title", true, false),
            ],
        );

        let json =
            serde_json::to_string_pretty(&primitive_type).expect("primitive type should serialize");
        let decoded: PrimitiveType =
            serde_json::from_str(&json).expect("primitive type should deserialize");

        assert_eq!(decoded, primitive_type);
        assert_eq!(
            decoded
                .field("title")
                .expect("title field should exist")
                .field_type,
            "string"
        );
    }

    #[test]
    fn builtin_registry_contains_all_required_types() {
        let registry = Registry::builtins();

        assert_eq!(registry.types.len(), 17);
        assert_eq!(
            registry
                .get_type("person")
                .expect("person should be builtin")
                .directory,
            "people"
        );
        assert_eq!(
            registry
                .get_type("strategic_note")
                .expect("strategic_note should be builtin")
                .directory,
            "strategic_notes"
        );
        assert!(registry.get_type("thread").is_some());
        assert!(registry.get_type("trigger").is_some());
        assert!(registry.get_type("checkpoint").is_some());
    }

    #[test]
    fn registry_roundtrips_with_builtins() {
        let registry = Registry::builtins();
        let json = serde_json::to_string_pretty(&registry).expect("registry should serialize");
        let decoded: Registry = serde_json::from_str(&json).expect("registry should deserialize");

        assert_eq!(decoded, registry);
        assert_eq!(decoded.list_types().len(), 17);
    }
}
