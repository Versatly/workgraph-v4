#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Wiki-link graph engine for WorkGraph primitives.
//!
//! This crate scans markdown primitives in a workspace, extracts `[[wiki-links]]`
//! from body and YAML frontmatter fields, and builds a directed graph between
//! source and target primitives.

mod build;
mod model;

pub use build::build_graph;
pub use model::{BrokenLink, Edge, GraphSnapshot, NeighborDirection, NodeRef};
