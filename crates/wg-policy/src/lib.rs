//! Placeholder policy evaluation primitives for WorkGraph.

#![forbid(unsafe_code)]

/// Minimal policy decision outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolicyDecision {
    /// The request is allowed.
    Allow,
    /// The request is denied.
    #[default]
    Deny,
}

/// Tiny policy evaluation input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PolicyCheck {
    /// Subject requesting access.
    pub subject: String,
    /// Action the subject wants to perform.
    pub action: String,
}

impl PolicyCheck {
    /// Evaluates the placeholder policy check.
    #[must_use]
    pub fn evaluate(&self) -> PolicyDecision {
        if self.subject.is_empty() || self.action.is_empty() {
            PolicyDecision::Deny
        } else {
            PolicyDecision::Allow
        }
    }
}
