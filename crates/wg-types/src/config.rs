//! Workspace configuration models shared across crates.

use crate::{ActorId, NodeId, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Describes the filesystem and identity configuration for a WorkGraph workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkgraphConfig {
    /// The stable workspace identifier.
    pub workspace_id: WorkspaceId,
    /// The human-readable workspace name.
    pub workspace_name: String,
    /// The filesystem root for the workspace.
    pub root_dir: String,
    /// The directory where markdown primitives are stored.
    pub store_dir: String,
    /// The metadata directory that holds registry, config, and ledger files.
    pub metadata_dir: String,
    /// The file path for the immutable ledger.
    pub ledger_file: String,
    /// The file path for the serialized registry definition.
    pub registry_file: String,
    /// The file path for this workspace configuration document.
    pub config_file: String,
    /// The default actor identifier to use for local CLI writes when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_actor_id: Option<ActorId>,
    /// The local node identifier when the workspace is part of a distributed deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_node_id: Option<NodeId>,
}

#[cfg(test)]
mod tests {
    use super::WorkgraphConfig;
    use crate::{ActorId, NodeId, WorkspaceId};

    #[test]
    fn workgraph_config_roundtrips_through_json() {
        let config = WorkgraphConfig {
            workspace_id: WorkspaceId::new("versatly"),
            workspace_name: "Versatly".into(),
            root_dir: "/workspace".into(),
            store_dir: "/workspace".into(),
            metadata_dir: "/workspace/.workgraph".into(),
            ledger_file: "/workspace/.workgraph/ledger.jsonl".into(),
            registry_file: "/workspace/.workgraph/registry.yaml".into(),
            config_file: "/workspace/.workgraph/config.yaml".into(),
            default_actor_id: Some(ActorId::new("cli")),
            local_node_id: Some(NodeId::new("node-a")),
        };

        let json = serde_json::to_string_pretty(&config).expect("config should serialize");
        let decoded: WorkgraphConfig =
            serde_json::from_str(&json).expect("config should deserialize");

        assert_eq!(decoded, config);
    }
}
