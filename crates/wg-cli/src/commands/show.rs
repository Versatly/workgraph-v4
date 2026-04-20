//! Implementation of the `workgraph show` command.

use anyhow::Context;
use wg_graph::{Edge, GraphSnapshot, NeighborDirection, NodeRef, build_graph};
use wg_store::read_primitive;

use crate::app::AppContext;
use crate::output::{PrimitiveReference, ShowOutput};
use crate::util::workspace::parse_reference;

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
        .map(|broken| PrimitiveReference {
            reference: broken.target.clone(),
            title: None,
            edge_kind: broken.kind.as_str().to_owned(),
            provenance: broken.provenance.as_str().to_owned(),
            direction: "broken".to_owned(),
            broken_reason: Some(broken.reason.clone()),
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
) -> Vec<PrimitiveReference> {
    graph
        .edges_for(node, direction)
        .into_iter()
        .map(|edge| reference_from_edge(&edge, direction))
        .collect()
}

fn reference_from_edge(edge: &Edge, direction: NeighborDirection) -> PrimitiveReference {
    let related = match direction {
        NeighborDirection::Inbound => &edge.source,
        NeighborDirection::Outbound => &edge.target,
    };

    PrimitiveReference {
        reference: related.reference(),
        title: None,
        edge_kind: edge.kind.as_str().to_owned(),
        provenance: edge.provenance.as_str().to_owned(),
        direction: match direction {
            NeighborDirection::Inbound => "inbound".to_owned(),
            NeighborDirection::Outbound => "outbound".to_owned(),
        },
        broken_reason: None,
    }
}
