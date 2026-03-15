//! Public document models used by the markdown primitive store.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// Typed YAML frontmatter stored at the top of a primitive Markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveFrontmatter {
    /// The stable primitive type name stored as the `type` frontmatter field.
    #[serde(rename = "type")]
    pub r#type: String,
    /// The stable primitive identifier.
    pub id: String,
    /// The human-readable primitive title.
    pub title: String,
    /// Additional frontmatter fields preserved alongside the required fields.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_fields: BTreeMap<String, Value>,
}

/// A primitive loaded from or ready to be written to the markdown store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPrimitive {
    /// The typed YAML frontmatter associated with the primitive.
    pub frontmatter: PrimitiveFrontmatter,
    /// The Markdown body stored after the closing frontmatter fence.
    pub body: String,
}

/// An exact-match scalar frontmatter filter expressed as `field=value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFilter {
    /// The frontmatter field name to compare.
    pub field: String,
    /// The exact scalar value to match after string conversion.
    pub value: String,
}
