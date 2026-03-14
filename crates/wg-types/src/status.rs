//! Explicit status machines for thread and run lifecycle models.

use serde::{Deserialize, Serialize};

/// Represents the lifecycle of a conversational or coordination thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    /// The thread exists but is still being defined.
    Draft,
    /// The thread is ready to begin active work.
    Ready,
    /// The thread is actively being worked.
    Active,
    /// The thread is waiting on external input.
    Waiting,
    /// The thread is blocked by a dependency or policy gate.
    Blocked,
    /// The thread has finished successfully.
    Done,
    /// The thread has been cancelled.
    Cancelled,
}

impl ThreadStatus {
    /// Returns whether a transition to `next` is allowed.
    ///
    /// Self-transitions are allowed to support idempotent writes.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (Self::Draft, Self::Ready | Self::Cancelled)
                    | (Self::Ready, Self::Active | Self::Cancelled)
                    | (
                        Self::Active,
                        Self::Waiting | Self::Blocked | Self::Done | Self::Cancelled
                    )
                    | (
                        Self::Waiting,
                        Self::Active | Self::Blocked | Self::Cancelled
                    )
                    | (
                        Self::Blocked,
                        Self::Active | Self::Waiting | Self::Cancelled
                    )
            )
    }

    /// Attempts to transition to `next`, returning an error message if the move is invalid.
    pub fn transition_to(self, next: Self) -> Result<Self, String> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(format!(
                "cannot transition thread status from {:?} to {:?}",
                self, next
            ))
        }
    }
}

/// Represents the execution lifecycle of an individual run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run has been accepted but has not started.
    Queued,
    /// The run is executing.
    Running,
    /// The run completed successfully.
    Succeeded,
    /// The run failed.
    Failed,
    /// The run exceeded its execution budget.
    TimedOut,
    /// The run was cancelled.
    Cancelled,
}

impl RunStatus {
    /// Returns whether a transition to `next` is allowed.
    ///
    /// Self-transitions are allowed to support idempotent writes.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (Self::Queued, Self::Running | Self::Cancelled)
                    | (
                        Self::Running,
                        Self::Succeeded | Self::Failed | Self::TimedOut | Self::Cancelled
                    )
                    | (Self::Failed, Self::Queued | Self::Cancelled)
                    | (Self::TimedOut, Self::Queued | Self::Cancelled)
            )
    }

    /// Attempts to transition to `next`, returning an error message if the move is invalid.
    pub fn transition_to(self, next: Self) -> Result<Self, String> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(format!(
                "cannot transition run status from {:?} to {:?}",
                self, next
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RunStatus, ThreadStatus};

    #[test]
    fn thread_status_allows_expected_transitions() {
        assert!(ThreadStatus::Draft.can_transition_to(ThreadStatus::Ready));
        assert!(ThreadStatus::Ready.can_transition_to(ThreadStatus::Active));
        assert!(ThreadStatus::Active.can_transition_to(ThreadStatus::Blocked));
        assert!(ThreadStatus::Blocked.can_transition_to(ThreadStatus::Waiting));
        assert!(ThreadStatus::Waiting.can_transition_to(ThreadStatus::Active));
        assert!(ThreadStatus::Done.can_transition_to(ThreadStatus::Done));
    }

    #[test]
    fn thread_status_rejects_invalid_transitions() {
        assert!(!ThreadStatus::Draft.can_transition_to(ThreadStatus::Done));
        assert!(ThreadStatus::Cancelled
            .transition_to(ThreadStatus::Active)
            .expect_err("cancelled should be terminal")
            .contains("cannot transition thread status"));
    }

    #[test]
    fn run_status_allows_expected_transitions() {
        assert!(RunStatus::Queued.can_transition_to(RunStatus::Running));
        assert!(RunStatus::Running.can_transition_to(RunStatus::Succeeded));
        assert!(RunStatus::Running.can_transition_to(RunStatus::TimedOut));
        assert!(RunStatus::Failed.can_transition_to(RunStatus::Queued));
        assert_eq!(
            RunStatus::TimedOut
                .transition_to(RunStatus::Queued)
                .expect("timed out runs should be retryable"),
            RunStatus::Queued
        );
    }

    #[test]
    fn run_status_rejects_invalid_transitions() {
        assert!(!RunStatus::Queued.can_transition_to(RunStatus::Succeeded));
        assert!(RunStatus::Succeeded
            .transition_to(RunStatus::Running)
            .expect_err("succeeded should be terminal")
            .contains("cannot transition run status"));
    }
}
