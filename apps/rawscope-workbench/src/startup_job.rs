//! Bounded startup-session resolution submitted to the native job coordinator.

use std::{
    io,
    sync::{Arc, Mutex},
};

use crate::{
    app_session::{resolve_workbench_startup, WorkbenchStartup},
    cli::WorkbenchArgs,
    job_coordinator::{
        CancellationToken, JobCoordinator, JobHandle, JobOutcome, JobSubmitError,
        WorkbenchJobGeneration, WorkbenchJobKind,
    },
};

#[derive(Clone)]
pub(crate) struct StartupResolution {
    result: Arc<Mutex<Option<Result<WorkbenchStartup, io::Error>>>>,
}

impl StartupResolution {
    pub(crate) fn take(&self) -> Option<Result<WorkbenchStartup, io::Error>> {
        self.result.lock().expect("startup result lock").take()
    }
}

pub(crate) fn submit_startup_resolution(
    coordinator: &JobCoordinator,
    args: WorkbenchArgs,
) -> Result<(JobHandle, StartupResolution), JobSubmitError> {
    let result = Arc::new(Mutex::new(None));
    let worker_result = Arc::clone(&result);
    let handle = coordinator.submit_with_metadata(
        WorkbenchJobKind::StartupResolution,
        WorkbenchJobGeneration(0),
        move |token: CancellationToken| {
            if token.is_cancelled() {
                return JobOutcome::Cancelled;
            }
            let resolved = resolve_workbench_startup(args);
            let outcome = if resolved.is_ok() {
                JobOutcome::Succeeded
            } else {
                JobOutcome::Failed
            };
            *worker_result.lock().expect("startup result lock") = Some(resolved);
            outcome
        },
    )?;
    Ok((handle, StartupResolution { result }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::WorkbenchArgs;

    #[test]
    fn startup_resolution_returns_owned_result_after_job_completion() {
        let coordinator = JobCoordinator::new(1);
        let args = WorkbenchArgs::parse(Vec::<String>::new()).unwrap();
        let (job, resolution) = submit_startup_resolution(&coordinator, args).unwrap();
        while job.outcome().is_none() {
            std::thread::yield_now();
        }
        assert!(resolution.take().unwrap().is_ok());
    }
}
