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
mod registry;
mod validate;

pub use document::{FieldFilter, FilterOperator, PrimitiveFrontmatter, StoredPrimitive};
pub use io::{
    AuditedWriteRequest, list_primitives, read_primitive, write_primitive, write_primitive_audited,
    write_primitive_audited_now,
};
pub use query::query_primitives;
pub use registry::load_workspace_registry;
pub use validate::validate_primitive;
