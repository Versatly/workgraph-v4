#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Policy loading and evaluation for WorkGraph.
//!
//! Policies are loaded from `policy` primitives and evaluated against actor,
//! action, and primitive type context. The initial engine focuses on actor-based
//! allow/deny rules for CRUD actions.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{StoredPrimitive, list_primitives};
use wg_types::ActorId;

/// Policy decision outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    /// The request is allowed.
    Allow,
    /// The request is denied.
    #[default]
    Deny,
}

/// Primitive mutation actions used for policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    /// Create a primitive.
    Create,
    /// Read a primitive.
    Read,
    /// Update a primitive.
    Update,
    /// Delete a primitive.
    Delete,
}

impl PolicyAction {
    /// Returns every supported action.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Create, Self::Read, Self::Update, Self::Delete]
    }
}

impl FromStr for PolicyAction {
    type Err = String;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "create" => Ok(Self::Create),
            "read" => Ok(Self::Read),
            "update" => Ok(Self::Update),
            "delete" => Ok(Self::Delete),
            _ => Err(format!("unsupported policy action '{input}'")),
        }
    }
}

/// Context bag for policy evaluation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolicyContext {
    /// Optional contextual fields attached to the evaluated request.
    pub fields: BTreeMap<String, Value>,
}

/// One policy rule inside a policy definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRule {
    /// Rule effect.
    pub effect: PolicyDecision,
    /// Allowed/denied actors. Empty means all actors.
    pub actors: Vec<ActorId>,
    /// Actions the rule applies to.
    pub actions: Vec<PolicyAction>,
    /// Primitive types in scope for this rule. Empty means inherited policy scope.
    pub primitive_types: Vec<String>,
}

/// Parsed policy definition loaded from a primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDefinition {
    /// Policy primitive identifier.
    pub id: String,
    /// Policy title.
    pub title: String,
    /// Primitive-type scope for this policy.
    pub scope: Vec<String>,
    /// Rules attached to this policy.
    pub rules: Vec<PolicyRule>,
}

/// Tiny compatibility policy evaluation input used by placeholder trigger crate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PolicyCheck {
    /// Subject requesting access.
    pub subject: String,
    /// Action the subject wants to perform.
    pub action: String,
}

impl PolicyCheck {
    /// Evaluates the compatibility policy check.
    #[must_use]
    pub fn evaluate(&self) -> PolicyDecision {
        if self.subject.is_empty() || self.action.is_empty() {
            PolicyDecision::Deny
        } else {
            PolicyDecision::Allow
        }
    }
}

/// In-memory policy engine with cached policy definitions.
#[derive(Debug, Clone, Default)]
pub struct PolicyEngine {
    policies: Vec<PolicyDefinition>,
}

impl PolicyEngine {
    /// Loads policies from workspace storage into a cached engine.
    ///
    /// # Errors
    ///
    /// Returns an error when policies cannot be loaded or parsed.
    pub async fn load(workspace: &WorkspacePath) -> Result<Self> {
        Ok(Self {
            policies: load_policies(workspace).await?,
        })
    }

    /// Replaces cached policies by reloading from workspace storage.
    ///
    /// # Errors
    ///
    /// Returns an error when policies cannot be loaded.
    pub async fn reload(&mut self, workspace: &WorkspacePath) -> Result<()> {
        self.policies = load_policies(workspace).await?;
        Ok(())
    }

    /// Returns cached policy definitions.
    #[must_use]
    pub fn policies(&self) -> &[PolicyDefinition] {
        &self.policies
    }

    /// Evaluates a request against cached policy definitions.
    ///
    /// Deny rules always win. If one or more allow rules apply to the requested
    /// action/type, actors not present in those allow rules are denied. If no
    /// applicable rules exist, the decision defaults to allow.
    #[must_use]
    pub fn evaluate(
        &self,
        actor: &ActorId,
        action: PolicyAction,
        primitive_type: &str,
        context: &PolicyContext,
    ) -> PolicyDecision {
        let mut any_allow_rule_for_scope = false;
        let mut allow_matched = false;

        for policy in &self.policies {
            for rule in &policy.rules {
                if !matches_action_and_type(rule, action, primitive_type, &policy.scope) {
                    continue;
                }

                if rule.effect == PolicyDecision::Allow {
                    any_allow_rule_for_scope = true;
                    if matches_actor(rule, actor, context) {
                        allow_matched = true;
                    }
                } else if matches_actor(rule, actor, context) {
                    return PolicyDecision::Deny;
                }
            }
        }

        if any_allow_rule_for_scope {
            if allow_matched {
                PolicyDecision::Allow
            } else {
                PolicyDecision::Deny
            }
        } else {
            PolicyDecision::Allow
        }
    }
}

/// Loads and parses policy primitives from workspace storage.
///
/// # Errors
///
/// Returns an error when policy primitives cannot be loaded or a policy cannot
/// be parsed.
pub async fn load_policies(workspace: &WorkspacePath) -> Result<Vec<PolicyDefinition>> {
    let primitives = list_primitives(workspace, "policy").await?;
    let mut policies = Vec::with_capacity(primitives.len());

    for primitive in primitives {
        policies.push(parse_policy_definition(&primitive)?);
    }

    Ok(policies)
}

/// Evaluates a request by loading policies from storage on demand.
///
/// # Errors
///
/// Returns an error when policies cannot be loaded.
pub async fn evaluate(
    workspace: &WorkspacePath,
    actor: &ActorId,
    action: PolicyAction,
    primitive_type: &str,
    context: &PolicyContext,
) -> Result<PolicyDecision> {
    let engine = PolicyEngine::load(workspace).await?;
    Ok(engine.evaluate(actor, action, primitive_type, context))
}

fn parse_policy_definition(primitive: &StoredPrimitive) -> Result<PolicyDefinition> {
    let scope = primitive
        .frontmatter
        .extra_fields
        .get("scope")
        .map_or_else(Vec::new, parse_string_list);
    let rules = primitive
        .frontmatter
        .extra_fields
        .get("rules")
        .map_or_else(|| Ok(Vec::new()), |value| parse_rules(value, &scope))?;

    Ok(PolicyDefinition {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        scope,
        rules,
    })
}

fn parse_rules(value: &Value, scope: &[String]) -> Result<Vec<PolicyRule>> {
    #[derive(Debug, Clone, Deserialize)]
    struct RawRule {
        effect: PolicyDecision,
        #[serde(default)]
        actors: Vec<String>,
        #[serde(default)]
        actions: Vec<PolicyAction>,
        #[serde(default)]
        primitive_types: Vec<String>,
    }

    let raw_rules = serde_yaml::from_value::<Vec<RawRule>>(value.clone()).map_err(|error| {
        WorkgraphError::EncodingError(format!("failed to decode policy rules from YAML: {error}"))
    })?;
    let mut parsed = Vec::with_capacity(raw_rules.len());

    for raw_rule in raw_rules {
        parsed.push(PolicyRule {
            effect: raw_rule.effect,
            actors: raw_rule.actors.into_iter().map(ActorId::new).collect(),
            actions: if raw_rule.actions.is_empty() {
                PolicyAction::all().to_vec()
            } else {
                raw_rule.actions
            },
            primitive_types: if raw_rule.primitive_types.is_empty() {
                scope.to_vec()
            } else {
                raw_rule.primitive_types
            },
        });
    }

    Ok(parsed)
}

fn parse_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Sequence(values) => values.iter().filter_map(string_value).map(str::to_owned).collect(),
        Value::Tagged(tagged) => parse_string_list(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Mapping(_) => Vec::new(),
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

fn matches_action_and_type(
    rule: &PolicyRule,
    action: PolicyAction,
    primitive_type: &str,
    policy_scope: &[String],
) -> bool {
    if !rule.actions.contains(&action) {
        return false;
    }

    let effective_scope = if rule.primitive_types.is_empty() {
        policy_scope
    } else {
        &rule.primitive_types
    };

    if effective_scope.is_empty() {
        return true;
    }

    effective_scope
        .iter()
        .any(|candidate| candidate == primitive_type || candidate == "*")
}

fn matches_actor(rule: &PolicyRule, actor: &ActorId, _context: &PolicyContext) -> bool {
    if rule.actors.is_empty() {
        return true;
    }

    rule.actors
        .iter()
        .any(|candidate| candidate.as_str() == actor.as_str() || candidate.as_str() == "*")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
    use wg_types::{ActorId, Registry};

    use crate::{PolicyAction, PolicyContext, PolicyDecision, PolicyEngine, evaluate, load_policies};

    fn policy_primitive(
        id: &str,
        title: &str,
        scope: Value,
        rules: Value,
    ) -> StoredPrimitive {
        StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "policy".to_owned(),
                id: id.to_owned(),
                title: title.to_owned(),
                extra_fields: BTreeMap::from([
                    ("scope".to_owned(), scope),
                    ("rules".to_owned(), rules),
                ]),
            },
            body: "Policy body".to_owned(),
        }
    }

    #[tokio::test]
    async fn load_policies_and_evaluate_with_allow_lists() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let scope = Value::Sequence(vec![Value::String("decision".to_owned())]);
        let rules = Value::Sequence(vec![
            serde_yaml::to_value(serde_yaml::Mapping::from_iter([
                (
                    Value::String("effect".to_owned()),
                    Value::String("allow".to_owned()),
                ),
                (
                    Value::String("actions".to_owned()),
                    Value::Sequence(vec![Value::String("create".to_owned())]),
                ),
                (
                    Value::String("actors".to_owned()),
                    Value::Sequence(vec![Value::String("pedro".to_owned())]),
                ),
            ]))
            .expect("allow rule should serialize"),
            serde_yaml::to_value(serde_yaml::Mapping::from_iter([
                (
                    Value::String("effect".to_owned()),
                    Value::String("deny".to_owned()),
                ),
                (
                    Value::String("actions".to_owned()),
                    Value::Sequence(vec![Value::String("create".to_owned())]),
                ),
                (
                    Value::String("actors".to_owned()),
                    Value::Sequence(vec![Value::String("intern".to_owned())]),
                ),
            ]))
            .expect("deny rule should serialize"),
        ]);
        let policy = policy_primitive("decision-create-policy", "Decision create policy", scope, rules);

        write_primitive(&workspace, &Registry::builtins(), &policy)
            .await
            .expect("policy primitive should write");

        let loaded = load_policies(&workspace)
            .await
            .expect("policies should load");
        assert_eq!(loaded.len(), 1);

        let engine = PolicyEngine::load(&workspace)
            .await
            .expect("policy engine should load");
        let context = PolicyContext::default();

        assert_eq!(
            engine.evaluate(
                &ActorId::new("pedro"),
                PolicyAction::Create,
                "decision",
                &context
            ),
            PolicyDecision::Allow
        );
        assert_eq!(
            engine.evaluate(&ActorId::new("ana"), PolicyAction::Create, "decision", &context),
            PolicyDecision::Deny
        );
        assert_eq!(
            engine.evaluate(
                &ActorId::new("intern"),
                PolicyAction::Create,
                "decision",
                &context
            ),
            PolicyDecision::Deny
        );
        assert_eq!(
            engine.evaluate(&ActorId::new("ana"), PolicyAction::Read, "decision", &context),
            PolicyDecision::Allow
        );
    }

    #[tokio::test]
    async fn evaluate_function_loads_policies_on_demand() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let rules = Value::Sequence(vec![serde_yaml::to_value(serde_yaml::Mapping::from_iter([
            (
                Value::String("effect".to_owned()),
                Value::String("deny".to_owned()),
            ),
            (
                Value::String("actions".to_owned()),
                Value::Sequence(vec![Value::String("delete".to_owned())]),
            ),
            (
                Value::String("actors".to_owned()),
                Value::Sequence(vec![Value::String("contractor".to_owned())]),
            ),
            (
                Value::String("primitive_types".to_owned()),
                Value::Sequence(vec![Value::String("project".to_owned())]),
            ),
        ]))
        .expect("rule should serialize")]);
        let policy = policy_primitive(
            "project-delete-policy",
            "Project delete policy",
            Value::String("project".to_owned()),
            rules,
        );
        write_primitive(&workspace, &Registry::builtins(), &policy)
            .await
            .expect("policy primitive should write");

        let decision = evaluate(
            &workspace,
            &ActorId::new("contractor"),
            PolicyAction::Delete,
            "project",
            &PolicyContext::default(),
        )
        .await
        .expect("evaluation should succeed");
        assert_eq!(decision, PolicyDecision::Deny);
    }
}
