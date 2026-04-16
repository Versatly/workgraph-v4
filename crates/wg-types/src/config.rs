//! Workspace configuration models shared across crates.

use crate::{ActorId, NodeId, RemoteAccessScope, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Describes how a local CLI profile reaches a hosted WorkGraph workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteWorkspaceConfig {
    /// Base URL for the hosted WorkGraph HTTP server.
    pub server_url: String,
    /// Bearer token used for authenticating remote requests.
    pub auth_token: String,
    /// Actor identity to attribute remote mutations to.
    pub actor_id: ActorId,
    /// Governance scope granted to this hosted credential.
    #[serde(default)]
    pub access_scope: RemoteAccessScope,
}

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
    /// Optional hosted-workspace connection profile used by remote CLI and MCP surfaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteWorkspaceConfig>,
}

#[cfg(test)]
mod tests {
    use super::{RemoteWorkspaceConfig, WorkgraphConfig};
    use crate::{ActorId, NodeId, RemoteAccessScope, WorkspaceId};

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
            remote: Some(RemoteWorkspaceConfig {
                server_url: "http://127.0.0.1:8787".into(),
                auth_token: "secret".into(),
                actor_id: ActorId::new("agent:cursor"),
                access_scope: RemoteAccessScope::Operate,
            }),
        };

        let json = serde_json::to_string_pretty(&config).expect("config should serialize");
        let decoded: WorkgraphConfig =
            serde_json::from_str(&json).expect("config should deserialize");

        assert_eq!(decoded, config);
    }
}
