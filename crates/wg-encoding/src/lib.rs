//! Markdown frontmatter parsing and YAML serialization helpers.

use serde::Serialize;
use serde::de::DeserializeOwned;
use wg_error::{Result, WorkgraphError};

/// Parses YAML frontmatter from markdown and returns `(frontmatter, body)`.
pub fn parse_frontmatter<T>(markdown: &str) -> Result<(T, String)>
where
    T: DeserializeOwned,
{
    let mut lines = markdown.lines();
    let first = lines.next().map(str::trim_end);
    if first != Some("---") {
        return Err(WorkgraphError::InvalidFrontmatter);
    }

    let mut yaml_lines = Vec::new();
    let mut found_end = false;

    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            found_end = true;
            break;
        }
        yaml_lines.push(line.trim_end_matches('\r'));
    }

    if !found_end {
        return Err(WorkgraphError::InvalidFrontmatter);
    }

    let yaml = yaml_lines.join("\n");
    let body = lines.collect::<Vec<_>>().join("\n");
    let frontmatter = serde_yaml::from_str::<T>(&yaml)?;
    Ok((frontmatter, body))
}

/// Serializes frontmatter and body into markdown with YAML frontmatter.
pub fn write_frontmatter<T>(frontmatter: &T, body: &str) -> Result<String>
where
    T: Serialize,
{
    let mut yaml = serde_yaml::to_string(frontmatter)?;
    if let Some(stripped) = yaml.strip_prefix("---\n") {
        yaml = stripped.to_owned();
    }
    if let Some(stripped) = yaml.strip_suffix("...\n") {
        yaml = stripped.to_owned();
    }
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }

    Ok(format!("---\n{yaml}---\n{body}"))
}

/// Converts a serializable value into a YAML string.
pub fn to_yaml_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    Ok(serde_yaml::to_string(value)?)
}

/// Parses a YAML string into a strongly typed value.
pub fn from_yaml_str<T>(yaml: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    Ok(serde_yaml::from_str(yaml)?)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Frontmatter {
        title: String,
        status: String,
    }

    #[test]
    fn parse_frontmatter_extracts_yaml_and_body() {
        let markdown = "---\ntitle: Example\nstatus: active\n---\nBody line\n";
        let (frontmatter, body): (Frontmatter, String) =
            parse_frontmatter(markdown).expect("frontmatter should parse");

        assert_eq!(frontmatter.title, "Example");
        assert_eq!(frontmatter.status, "active");
        assert_eq!(body, "Body line");
    }

    #[test]
    fn write_frontmatter_round_trips() {
        let frontmatter = Frontmatter {
            title: "Example".to_owned(),
            status: "active".to_owned(),
        };
        let markdown = write_frontmatter(&frontmatter, "Hello").expect("write should succeed");
        let (decoded, body): (Frontmatter, String) =
            parse_frontmatter(&markdown).expect("parse should succeed");

        assert_eq!(decoded, frontmatter);
        assert_eq!(body, "Hello");
    }
}
