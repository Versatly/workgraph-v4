#![forbid(unsafe_code)]

//! Obsidian integration placeholders.

/// Describes an Obsidian vault targeted for sync.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObsidianVault<'a> {
    /// Human-readable vault name.
    pub name: &'a str,
}

/// Placeholder sync service for Obsidian vaults.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ObsidianSync;

impl ObsidianSync {
    /// Creates a new placeholder Obsidian sync service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns whether the placeholder integration accepts the vault.
    pub fn supports(&self, _vault: ObsidianVault<'_>) -> bool {
        true
    }
}
