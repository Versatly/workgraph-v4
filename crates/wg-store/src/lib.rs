//! Markdown-backed primitive storage and query operations.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use wg_encoding::{parse_frontmatter, write_frontmatter};
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_types::PrimitiveType;

/// Frontmatter schema for stored primitives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveFrontmatter {
    /// Primitive type.
    #[serde(rename = "type")]
    pub primitive_type: PrimitiveType,
    /// Primitive ID.
    pub id: String,
    /// Primitive title.
    pub title: String,
    /// Arbitrary additional fields.
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}

/// Primitive markdown document representation.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredPrimitive {
    /// Parsed frontmatter.
    pub frontmatter: PrimitiveFrontmatter,
    /// Markdown body content.
    pub body: String,
}

impl StoredPrimitive {
    /// Creates a new stored primitive value.
    #[must_use]
    pub fn new(frontmatter: PrimitiveFrontmatter, body: impl Into<String>) -> Self {
        Self {
            frontmatter,
            body: body.into(),
        }
    }
}

/// Reads a primitive markdown file from disk.
pub fn read_primitive(
    workspace: &WorkspacePath,
    primitive_type: PrimitiveType,
    id: &str,
) -> Result<StoredPrimitive> {
    let file = workspace.store_dir_for(&primitive_type).primitive_file(id);
    let content = fs::read_to_string(&file).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            WorkgraphError::NotFound {
                primitive_type: primitive_type.as_str().to_owned(),
                id: id.to_owned(),
            }
        } else {
            WorkgraphError::Io(error)
        }
    })?;

    let (frontmatter, body): (PrimitiveFrontmatter, String) = parse_frontmatter(&content)?;
    validate_frontmatter(&frontmatter, &primitive_type, id)?;
    Ok(StoredPrimitive::new(frontmatter, body))
}

/// Writes a primitive markdown file to disk.
pub fn write_primitive(workspace: &WorkspacePath, primitive: &StoredPrimitive) -> Result<()> {
    let store = workspace.store_dir_for(&primitive.frontmatter.primitive_type);
    fs::create_dir_all(store.as_path())?;
    let file = store.primitive_file(&primitive.frontmatter.id);

    let markdown = write_frontmatter(&primitive.frontmatter, &primitive.body)?;
    fs::write(file, markdown)?;
    Ok(())
}

/// Lists all primitives for a given type.
pub fn list_primitives(
    workspace: &WorkspacePath,
    primitive_type: PrimitiveType,
) -> Result<Vec<StoredPrimitive>> {
    let store = workspace.store_dir_for(&primitive_type);
    if !store.as_path().exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(store.as_path())? {
        let entry = entry?;
        let path = entry.path();
        if !is_markdown_file(&path) {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let (frontmatter, body): (PrimitiveFrontmatter, String) = parse_frontmatter(&content)?;
        if frontmatter.primitive_type == primitive_type {
            items.push(StoredPrimitive::new(frontmatter, body));
        }
    }

    items.sort_by(|left, right| left.frontmatter.id.cmp(&right.frontmatter.id));
    Ok(items)
}

/// Queries primitives of a type by exact key/value filters.
pub fn query(
    workspace: &WorkspacePath,
    primitive_type: PrimitiveType,
    filters: &BTreeMap<String, String>,
) -> Result<Vec<StoredPrimitive>> {
    let primitives = list_primitives(workspace, primitive_type)?;

    let mut results = Vec::new();
    for primitive in primitives {
        if filters
            .iter()
            .all(|(key, expected)| matches_filter(&primitive.frontmatter, key, expected))
        {
            results.push(primitive);
        }
    }

    Ok(results)
}

fn validate_frontmatter(
    frontmatter: &PrimitiveFrontmatter,
    expected_type: &PrimitiveType,
    expected_id: &str,
) -> Result<()> {
    if &frontmatter.primitive_type != expected_type {
        return Err(WorkgraphError::Validation(format!(
            "frontmatter type mismatch: expected {}, found {}",
            expected_type, frontmatter.primitive_type
        )));
    }

    if frontmatter.id != expected_id {
        return Err(WorkgraphError::Validation(format!(
            "frontmatter id mismatch: expected {expected_id}, found {}",
            frontmatter.id
        )));
    }

    Ok(())
}

fn is_markdown_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md")
}

fn matches_filter(frontmatter: &PrimitiveFrontmatter, key: &str, expected: &str) -> bool {
    match key {
        "type" => frontmatter.primitive_type.as_str() == expected,
        "id" => frontmatter.id == expected,
        "title" => frontmatter.title == expected,
        other => frontmatter
            .fields
            .get(other)
            .is_some_and(|value| value_matches(value, expected)),
    }
}

fn value_matches(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(v) => v == expected,
        Value::Bool(v) => v.to_string() == expected,
        Value::Number(v) => v.to_string() == expected,
        Value::Sequence(items) => items.iter().any(|item| value_matches(item, expected)),
        Value::Null => expected.eq_ignore_ascii_case("null"),
        Value::Mapping(_) => false,
        Value::Tagged(tagged) => value_matches(&tagged.value, expected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_primitive(primitive_type: PrimitiveType, id: &str, title: &str) -> StoredPrimitive {
        StoredPrimitive::new(
            PrimitiveFrontmatter {
                primitive_type,
                id: id.to_owned(),
                title: title.to_owned(),
                fields: BTreeMap::new(),
            },
            "Body",
        )
    }

    #[test]
    fn write_then_read_round_trips() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let workspace = WorkspacePath::new(tempdir.path());
        let primitive = fixture_primitive(PrimitiveType::Org, "acme", "Acme");

        write_primitive(&workspace, &primitive).expect("write should succeed");
        let loaded = read_primitive(&workspace, PrimitiveType::Org, "acme")
            .expect("primitive should be readable");

        assert_eq!(loaded, primitive);
    }

    #[test]
    fn query_filters_by_fields() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let workspace = WorkspacePath::new(tempdir.path());

        let mut org = fixture_primitive(PrimitiveType::Org, "acme", "Acme");
        org.frontmatter
            .fields
            .insert("region".to_owned(), Value::String("us".to_owned()));
        write_primitive(&workspace, &org).expect("org should be written");

        let client = fixture_primitive(PrimitiveType::Client, "globex", "Globex");
        write_primitive(&workspace, &client).expect("client should be written");

        let mut filters = BTreeMap::new();
        filters.insert("region".to_owned(), "us".to_owned());

        let results = query(&workspace, PrimitiveType::Org, &filters).expect("query should work");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].frontmatter.id, "acme");
    }
}
