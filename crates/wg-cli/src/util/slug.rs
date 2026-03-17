//! Helpers for deriving stable CLI-friendly slugs and identifiers.

use anyhow::Context;
use tokio::fs;
use wg_paths::WorkspacePath;

/// Normalizes a human title into a lowercase, dash-delimited identifier slug.
#[must_use]
pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "untitled".to_owned()
    } else {
        slug
    }
}

/// Generates a unique primitive identifier by suffixing collisions with incrementing counters.
///
/// # Errors
///
/// Returns an error when the filesystem cannot be checked for existing primitive paths.
pub async fn unique_slug(
    workspace: &WorkspacePath,
    primitive_type: &str,
    title: &str,
) -> anyhow::Result<String> {
    let base = slugify(title);
    let mut candidate = base.clone();
    let mut suffix = 2_usize;

    loop {
        let exists = fs::try_exists(
            workspace
                .primitive_path(primitive_type, &candidate)
                .as_path(),
        )
        .await
        .with_context(|| {
            format!(
                "failed to inspect primitive path for type '{primitive_type}' and id '{candidate}'"
            )
        })?;

        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}
