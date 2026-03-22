#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Typed graph snapshot builder for WorkGraph primitives.
//!
//! This crate scans markdown primitives in a workspace and emits a graph that
//! combines loose wiki references with stronger coordination edges derived from
//! structured fields such as assignments, mission containment, trigger plans,
//! and thread evidence.

mod build;
mod model;

pub use build::build_graph;
pub use model::{BrokenLink, Edge, GraphSnapshot, NeighborDirection, NodeRef};
