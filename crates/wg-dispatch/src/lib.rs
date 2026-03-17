//! Placeholder execution dispatch primitives for WorkGraph.

#![forbid(unsafe_code)]

use wg_mission::MissionPlan;
use wg_thread::ThreadHandle;

/// Minimal request passed into dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DispatchRequest {
    /// Mission to execute.
    pub mission: MissionPlan,
    /// Thread that will carry the execution.
    pub thread: ThreadHandle,
}

/// Creates a placeholder dispatch request.
#[must_use]
pub fn prepare_dispatch(mission: MissionPlan, thread: ThreadHandle) -> DispatchRequest {
    DispatchRequest { mission, thread }
}
