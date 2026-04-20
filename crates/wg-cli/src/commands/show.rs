//! Implementation of the `workgraph show` command.

use anyhow::Context;
use wg_graph::{Edge, GraphSnapshot, NeighborDirection, NodeRef, build_graph};
use wg_store::read_primitive;

use crate::app::AppContext;
use crate::output::{GraphReferenceOutput, ShowOutput};
use crate::util::workspace::parse_reference;
use wg_orientation::GraphIssue;

/// Loads and returns a single primitive by `<type>/<id>` reference.
///
/// # Errors
///
/// Returns an error when the reference is invalid or the primitive cannot be read.
pub async fn handle(app: &AppContext, reference: &str) -> anyhow::Result<ShowOutput> {
    let (primitive_type, id) = parse_reference(reference)?;
    let primitive = read_primitive(app.workspace(), primitive_type, id)
        .await
        .with_context(|| format!("failed to read primitive '{reference}'"))?;
    let graph = build_graph(app.workspace()).await?;
    let node = NodeRef::new(primitive_type, id);
    let inbound_references = references_for(&graph, &node, NeighborDirection::Inbound);
    let outbound_references = references_for(&graph, &node, NeighborDirection::Outbound);
    let broken_references = graph
        .broken_links()
        .iter()
        .filter(|broken| broken.source == node)
        .map(|broken| GraphIssue {
            source_reference: broken.source.reference(),
            target_reference: broken.target.clone(),
            kind: edge_kind_text(broken.kind).to_owned(),
            provenance: edge_source_text(broken.provenance).to_owned(),
            reason: broken.reason.clone(),
        })
        .collect();

    Ok(ShowOutput {
        reference: reference.to_owned(),
        primitive,
        inbound_references,
        outbound_references,
        broken_references,
    })
}

fn references_for(
    graph: &GraphSnapshot,
    node: &NodeRef,
    direction: NeighborDirection,
) -> Vec<GraphReferenceOutput> {
    graph
        .edge_refs(node, direction)
        .into_iter()
        .map(|edge| reference_from_edge(&edge, direction))
        .collect()
}

fn reference_from_edge(edge: &Edge, _direction: NeighborDirection) -> GraphReferenceOutput {
    GraphReferenceOutput::from_edge(edge)
}

fn edge_kind_text(kind: wg_types::GraphEdgeKind) -> &'static str {
    match kind {
        wg_types::GraphEdgeKind::Reference => "reference",
        wg_types::GraphEdgeKind::Relationship => "relationship",
        wg_types::GraphEdgeKind::Assignment => "assignment",
        wg_types::GraphEdgeKind::Containment => "containment",
        wg_types::GraphEdgeKind::Evidence => "evidence",
        wg_types::GraphEdgeKind::Trigger => "trigger",
    }
}

fn edge_source_text(source: wg_types::GraphEdgeSource) -> &'static str {
    match source {
        wg_types::GraphEdgeSource::WikiLink => "wiki_link",
        wg_types::GraphEdgeSource::Field => "field",
        wg_types::GraphEdgeSource::RelationshipPrimitive => "relationship_primitive",
        wg_types::GraphEdgeSource::EvidenceRecord => "evidence_record",
        wg_types::GraphEdgeSource::TriggerRule => "trigger_rule",
    }
}
