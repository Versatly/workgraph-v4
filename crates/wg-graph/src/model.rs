//! Graph data model and traversal helpers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Identifies a primitive node in the workspace graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRef {
    /// Primitive type of the node, for example `decision`.
    pub primitive_type: String,
    /// Primitive identifier of the node.
    pub id: String,
}

impl NodeRef {
    /// Creates a node reference from a primitive type and identifier.
    #[must_use]
    pub fn new(primitive_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            primitive_type: primitive_type.into(),
            id: id.into(),
        }
    }

    /// Parses a `type/id` reference into a node reference.
    #[must_use]
    pub fn from_reference(reference: &str) -> Option<Self> {
        let (primitive_type, id) = reference.split_once('/')?;
        let primitive_type = primitive_type.trim();
        let id = id.trim();

        if primitive_type.is_empty() || id.is_empty() {
            return None;
        }

        Some(Self::new(primitive_type, id))
    }

    /// Returns a stable `type/id` reference.
    #[must_use]
    pub fn reference(&self) -> String {
        format!("{}/{}", self.primitive_type, self.id)
    }
}

/// A directed edge between two graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    /// Source primitive.
    pub source: NodeRef,
    /// Target primitive.
    pub target: NodeRef,
}

/// Direction for neighbor traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborDirection {
    /// Traverse inbound edges.
    Inbound,
    /// Traverse outbound edges.
    Outbound,
}

/// A broken wiki-link discovered during graph construction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrokenLink {
    /// Primitive containing the unresolved link.
    pub source: NodeRef,
    /// Raw normalized link target from the markdown or YAML field.
    pub target: String,
    /// Human-readable reason the link could not be resolved.
    pub reason: String,
}

/// Immutable directed graph snapshot for a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphSnapshot {
    nodes: BTreeSet<NodeRef>,
    edges: BTreeSet<Edge>,
    outbound: BTreeMap<NodeRef, BTreeSet<NodeRef>>,
    inbound: BTreeMap<NodeRef, BTreeSet<NodeRef>>,
    broken_links: Vec<BrokenLink>,
}

impl GraphSnapshot {
    pub(crate) fn from_parts(
        nodes: BTreeSet<NodeRef>,
        edges: BTreeSet<Edge>,
        broken_links: Vec<BrokenLink>,
    ) -> Self {
        let mut outbound: BTreeMap<NodeRef, BTreeSet<NodeRef>> = BTreeMap::new();
        let mut inbound: BTreeMap<NodeRef, BTreeSet<NodeRef>> = BTreeMap::new();

        for edge in &edges {
            outbound
                .entry(edge.source.clone())
                .or_default()
                .insert(edge.target.clone());
            inbound
                .entry(edge.target.clone())
                .or_default()
                .insert(edge.source.clone());
        }

        Self {
            nodes,
            edges,
            outbound,
            inbound,
            broken_links,
        }
    }

    /// Returns `true` when the graph has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns all nodes in sorted `type/id` order.
    #[must_use]
    pub fn nodes(&self) -> Vec<NodeRef> {
        self.nodes.iter().cloned().collect()
    }

    /// Returns all directed edges in sorted source/target order.
    #[must_use]
    pub fn edges(&self) -> Vec<Edge> {
        self.edges.iter().cloned().collect()
    }

    /// Returns neighboring nodes for the provided node and direction.
    #[must_use]
    pub fn neighbors(&self, node: &NodeRef, direction: NeighborDirection) -> Vec<NodeRef> {
        let table = match direction {
            NeighborDirection::Inbound => &self.inbound,
            NeighborDirection::Outbound => &self.outbound,
        };

        table
            .get(node)
            .map_or_else(Vec::new, |nodes| nodes.iter().cloned().collect())
    }

    /// Returns all nodes reachable from `start` by following outbound links.
    #[must_use]
    pub fn reachable(&self, start: &NodeRef) -> Vec<NodeRef> {
        if !self.nodes.contains(start) {
            return Vec::new();
        }

        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start.clone());

        while let Some(current) = queue.pop_front() {
            for next in self.neighbors(&current, NeighborDirection::Outbound) {
                if visited.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }

        visited.into_iter().collect()
    }

    /// Returns nodes with no inbound links.
    #[must_use]
    pub fn orphans(&self) -> Vec<NodeRef> {
        self.nodes
            .iter()
            .filter(|node| !self.inbound.contains_key(*node))
            .cloned()
            .collect()
    }

    /// Returns links that point to non-existent or ambiguous targets.
    #[must_use]
    pub fn broken_links(&self) -> &[BrokenLink] {
        &self.broken_links
    }
}
