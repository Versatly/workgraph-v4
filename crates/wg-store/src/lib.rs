#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Markdown-native primitive storage, validation, and querying for WorkGraph.
//!
//! This crate owns the filesystem-backed primitive store used by the WorkGraph
//! kernel. Primitives are persisted as Markdown documents with YAML
//! frontmatter, stored under pluralized per-type directories.
mod document;
mod io;
mod query;
mod validate;

pub use document::{FieldFilter, PrimitiveFrontmatter, StoredPrimitive};
pub use io::{list_primitives, read_primitive, write_primitive};
pub use query::query_primitives;
pub use validate::validate_primitive;
