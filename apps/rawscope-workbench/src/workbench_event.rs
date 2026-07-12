//! Typed events used to wake the workbench owner after background work.

use crate::job_coordinator::{JobOutcome, WorkbenchJobId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkbenchUserEvent {
    JobCompleted {
        job_id: WorkbenchJobId,
        outcome: JobOutcome,
    },
}
