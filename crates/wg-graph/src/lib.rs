//! Placeholder graph traversal primitives for WorkGraph.

#![forbid(unsafe_code)]

/// Identifies a graph node in placeholder APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct NodeRef(pub String);

impl NodeRef {
    /// Creates a new node reference.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Minimal directed edge between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Edge {
    /// Source node of the edge.
    pub source: NodeRef,
    /// Target node of the edge.
    pub target: NodeRef,
}

/// Snapshot of a tiny placeholder graph.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphSnapshot {
    /// Directed edges captured in the snapshot.
    pub edges: Vec<Edge>,
}

impl GraphSnapshot {
    /// Returns true when the snapshot contains no edges.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}
