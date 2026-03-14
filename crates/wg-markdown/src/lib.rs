#![forbid(unsafe_code)]

//! Markdown projection placeholders for human-readable workspace output.

/// Minimal markdown document handled by the placeholder writer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownDocument<'a> {
    /// Title rendered as the primary heading.
    pub title: &'a str,
    /// Body content rendered below the heading.
    pub body: &'a str,
}

/// Placeholder markdown writer for projection output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownWriter;

impl MarkdownWriter {
    /// Creates a new placeholder markdown writer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns the heading that would be rendered for the document.
    pub fn heading<'a>(&self, document: &'a MarkdownDocument<'a>) -> &'a str {
        document.title
    }
}
