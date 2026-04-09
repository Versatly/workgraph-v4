//! Stable identity wrappers used across WorkGraph models.

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/// Identifies a durable tracked actor accountable for work.
///
/// `ActorId` is the stable logical identity boundary used across coordination,
/// graph, and ledger surfaces. In the current foundation pass, tracked actors
/// are usually materialized through `person` and `agent` primitives.
///
/// Runtime sessions, chat surfaces, spawned subagents, and other short-lived
/// execution details may be recorded as metadata elsewhere without requiring a
/// new first-class actor node for every ephemeral descendant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(String);

impl ActorId {
    /// Creates a new actor identifier.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ActorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for ActorId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ActorId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Identifies a WorkGraph workspace instance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Creates a new workspace identifier.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for WorkspaceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for WorkspaceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorkspaceId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Identifies a node participating in a WorkGraph deployment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    /// Creates a new node identifier.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{ActorId, NodeId, WorkspaceId};

    #[test]
    fn identity_helpers_expose_inner_values() {
        let actor = ActorId::new("pedro");
        let workspace = WorkspaceId::new("versatly");
        let node = NodeId::new("node-a");

        assert_eq!(actor.as_str(), "pedro");
        assert_eq!(workspace.to_string(), "versatly");
        assert_eq!(node.to_string(), "node-a");
    }

    #[test]
    fn identity_types_serialize_as_plain_strings() {
        let actor = ActorId::new("clawdious");
        let json = serde_json::to_string(&actor).expect("actor id should serialize");
        let decoded: ActorId = serde_json::from_str(&json).expect("actor id should deserialize");

        assert_eq!(json, "\"clawdious\"");
        assert_eq!(decoded, actor);
    }
}
