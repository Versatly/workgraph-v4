//! Helpers for deriving stable CLI-friendly slugs and identifiers.

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

/// Validates an explicit identifier or derives one deterministically from the title.
///
/// # Errors
///
/// Returns an error when the explicit identifier is empty or normalizes to a different stable slug.
pub fn validate_or_derive_id(id: Option<&str>, title: &str) -> anyhow::Result<String> {
    match id {
        Some(id) => {
            let trimmed = id.trim();
            if trimmed.is_empty() {
                anyhow::bail!("explicit --id must not be empty");
            }

            let normalized = slugify(trimmed);
            if normalized != trimmed {
                anyhow::bail!(
                    "explicit --id must already be a stable lowercase slug such as '{}'",
                    normalized
                );
            }

            Ok(trimmed.to_owned())
        }
        None => Ok(slugify(title)),
    }
}

