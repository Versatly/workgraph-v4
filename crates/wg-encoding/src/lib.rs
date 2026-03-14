#![forbid(unsafe_code)]

//! Markdown frontmatter parsing and serialization helpers.
//!
//! This crate provides strongly typed helpers for reading and writing
//! `---`-delimited YAML frontmatter at the top of Markdown documents.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::de::DeserializeOwned;
use serde::Serialize;

/// A Markdown document split into typed frontmatter and body content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterDocument<T> {
    /// The parsed YAML frontmatter.
    pub frontmatter: T,
    /// The Markdown body after the closing frontmatter fence.
    pub body: String,
}

/// Errors returned when parsing or serializing Markdown frontmatter.
#[derive(Debug)]
pub enum FrontmatterError {
    /// The document does not begin with an opening `---` fence.
    MissingOpeningFence,
    /// The document starts with an opening fence but never closes it.
    MissingClosingFence,
    /// The frontmatter section exists but contains no YAML content.
    EmptyFrontmatter,
    /// The YAML frontmatter could not be deserialized into the requested type.
    MalformedYaml(serde_yaml::Error),
    /// The provided value could not be serialized as YAML.
    SerializeYaml(serde_yaml::Error),
}

impl Display for FrontmatterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOpeningFence => {
                f.write_str("document is missing the opening frontmatter fence")
            }
            Self::MissingClosingFence => {
                f.write_str("document is missing the closing frontmatter fence")
            }
            Self::EmptyFrontmatter => f.write_str("frontmatter section is empty"),
            Self::MalformedYaml(error) => write!(f, "frontmatter YAML is malformed: {error}"),
            Self::SerializeYaml(error) => {
                write!(f, "frontmatter YAML could not be serialized: {error}")
            }
        }
    }
}

impl Error for FrontmatterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MalformedYaml(error) | Self::SerializeYaml(error) => Some(error),
            Self::MissingOpeningFence | Self::MissingClosingFence | Self::EmptyFrontmatter => None,
        }
    }
}

/// Parses a Markdown document with `---`-delimited YAML frontmatter.
///
/// The document must begin with an opening `---` fence on the first line and
/// contain a matching closing `---` fence on its own line. The YAML between the
/// fences is deserialized into `T`, while everything after the closing fence is
/// returned as the body.
///
/// # Errors
///
/// Returns [`FrontmatterError::MissingOpeningFence`] when the document does not
/// start with frontmatter, [`FrontmatterError::MissingClosingFence`] when the
/// frontmatter is never closed, [`FrontmatterError::EmptyFrontmatter`] when the
/// fenced YAML section is blank, and [`FrontmatterError::MalformedYaml`] when
/// deserialization fails.
pub fn parse_frontmatter<T>(document: &str) -> Result<FrontmatterDocument<T>, FrontmatterError>
where
    T: DeserializeOwned,
{
    let opening = read_line(document, 0).ok_or(FrontmatterError::MissingOpeningFence)?;
    if opening.text(document) != "---" {
        return Err(FrontmatterError::MissingOpeningFence);
    }

    let mut cursor = opening.next;
    while let Some(line) = read_line(document, cursor) {
        if line.text(document) == "---" {
            let raw_frontmatter = &document[opening.next..line.start];
            if raw_frontmatter.trim().is_empty() {
                return Err(FrontmatterError::EmptyFrontmatter);
            }

            let frontmatter =
                serde_yaml::from_str(raw_frontmatter).map_err(FrontmatterError::MalformedYaml)?;
            let body = document[line.next..].to_owned();

            return Ok(FrontmatterDocument { frontmatter, body });
        }

        cursor = line.next;
    }

    Err(FrontmatterError::MissingClosingFence)
}

/// Serializes typed YAML frontmatter and a Markdown body into one document.
///
/// The resulting document always uses `---` fences and includes a newline after
/// the closing fence. The body is appended verbatim and may be empty.
///
/// # Errors
///
/// Returns [`FrontmatterError::SerializeYaml`] when the provided frontmatter
/// value cannot be serialized to YAML.
pub fn write_frontmatter<T>(frontmatter: &T, body: &str) -> Result<String, FrontmatterError>
where
    T: Serialize,
{
    let serialized = serde_yaml::to_string(frontmatter).map_err(FrontmatterError::SerializeYaml)?;
    let serialized = strip_yaml_document_marker(&serialized);
    let serialized =
        serialized.trim_end_matches(|character| character == '\n' || character == '\r');

    if serialized.trim().is_empty() {
        return Err(FrontmatterError::EmptyFrontmatter);
    }

    let mut document = String::with_capacity(serialized.len() + body.len() + 9);
    document.push_str("---\n");
    document.push_str(serialized);
    document.push_str("\n---\n");
    document.push_str(body);
    Ok(document)
}

#[derive(Debug, Clone, Copy)]
struct LineRange {
    start: usize,
    end: usize,
    next: usize,
}

impl LineRange {
    fn text<'input>(&self, input: &'input str) -> &'input str {
        &input[self.start..self.end]
    }
}

fn read_line(input: &str, start: usize) -> Option<LineRange> {
    if start >= input.len() {
        return None;
    }

    let bytes = input.as_bytes();
    let mut end = start;
    while end < input.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
        end += 1;
    }

    let mut next = end;
    if next < input.len() {
        if bytes[next] == b'\r' {
            next += 1;
            if next < input.len() && bytes[next] == b'\n' {
                next += 1;
            }
        } else {
            next += 1;
        }
    }

    Some(LineRange { start, end, next })
}

fn strip_yaml_document_marker(serialized: &str) -> &str {
    serialized
        .strip_prefix("---\r\n")
        .or_else(|| serialized.strip_prefix("---\n"))
        .unwrap_or(serialized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct TestFrontmatter {
        title: String,
        tags: Vec<String>,
        draft: bool,
    }

    fn sample_frontmatter() -> TestFrontmatter {
        TestFrontmatter {
            title: "WorkGraph".to_owned(),
            tags: vec!["rust".to_owned(), "graph".to_owned()],
            draft: false,
        }
    }

    #[test]
    fn parse_frontmatter_reads_typed_yaml_and_body() {
        let document = "---\n\
title: WorkGraph\n\
tags:\n\
  - rust\n\
  - graph\n\
draft: false\n\
---\n\
# Heading\n\
\n\
Body text.\n";

        let parsed = parse_frontmatter::<TestFrontmatter>(document).unwrap();

        assert_eq!(parsed.frontmatter, sample_frontmatter());
        assert_eq!(parsed.body, "# Heading\n\nBody text.\n");
    }

    #[test]
    fn parse_frontmatter_supports_windows_line_endings() {
        let document = "---\r\n\
title: WorkGraph\r\n\
tags:\r\n\
  - rust\r\n\
  - graph\r\n\
draft: false\r\n\
---\r\n\
Body\r\n";

        let parsed = parse_frontmatter::<TestFrontmatter>(document).unwrap();

        assert_eq!(parsed.frontmatter, sample_frontmatter());
        assert_eq!(parsed.body, "Body\r\n");
    }

    #[test]
    fn parse_frontmatter_rejects_documents_without_opening_fence() {
        let error =
            parse_frontmatter::<TestFrontmatter>("title: WorkGraph\n---\nBody\n").unwrap_err();

        assert!(matches!(error, FrontmatterError::MissingOpeningFence));
    }

    #[test]
    fn parse_frontmatter_rejects_documents_without_closing_fence() {
        let error =
            parse_frontmatter::<TestFrontmatter>("---\ntitle: WorkGraph\nbody: missing fence\n")
                .unwrap_err();

        assert!(matches!(error, FrontmatterError::MissingClosingFence));
    }

    #[test]
    fn parse_frontmatter_rejects_empty_frontmatter() {
        let error = parse_frontmatter::<TestFrontmatter>("---\n\n---\nBody\n").unwrap_err();

        assert!(matches!(error, FrontmatterError::EmptyFrontmatter));
    }

    #[test]
    fn parse_frontmatter_accepts_an_empty_body() {
        let document = "---\n\
title: WorkGraph\n\
tags:\n\
  - rust\n\
  - graph\n\
draft: false\n\
---\n";

        let parsed = parse_frontmatter::<TestFrontmatter>(document).unwrap();

        assert_eq!(parsed.frontmatter, sample_frontmatter());
        assert!(parsed.body.is_empty());
    }

    #[test]
    fn parse_frontmatter_reports_malformed_yaml() {
        let error = parse_frontmatter::<TestFrontmatter>(
            "---\n\
title: WorkGraph\n\
tags: [rust\n\
draft: false\n\
---\n\
Body\n",
        )
        .unwrap_err();

        assert!(matches!(error, FrontmatterError::MalformedYaml(_)));
        assert!(error.to_string().contains("malformed"));
    }

    #[test]
    fn write_frontmatter_serializes_and_roundtrips() {
        let frontmatter = sample_frontmatter();
        let body = "# Heading\n\nBody text.\n";

        let document = write_frontmatter(&frontmatter, body).unwrap();
        let parsed = parse_frontmatter::<TestFrontmatter>(&document).unwrap();

        assert_eq!(parsed.frontmatter, frontmatter);
        assert_eq!(parsed.body, body);
        assert!(document.starts_with("---\n"));
    }

    #[test]
    fn write_frontmatter_supports_empty_body() {
        let document = write_frontmatter(&sample_frontmatter(), "").unwrap();

        assert!(document.ends_with("\n---\n"));

        let parsed = parse_frontmatter::<TestFrontmatter>(&document).unwrap();
        assert!(parsed.body.is_empty());
    }
}
