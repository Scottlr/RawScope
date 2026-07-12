//! Bounded asynchronous export lifecycle, independent of the UI facade.

use std::sync::Arc;

use rawscope_core::SelectionId;

use crate::job_coordinator::{
    CancellationToken, JobCoordinator, JobHandle, JobOutcome, JobSubmitError,
    WorkbenchJobGeneration, WorkbenchJobId, WorkbenchJobKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExportRequest {
    pub(crate) selection: SelectionId,
    pub(crate) evidence_generation: u64,
    pub(crate) bundle_label: Arc<str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedBundleSummary {
    pub(crate) selection: SelectionId,
    pub(crate) evidence_generation: u64,
    pub(crate) bundle_label: Arc<str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExportControllerError {
    Busy,
    QueueFull,
    ShuttingDown,
    Failed,
    Cancelled,
    Panicked,
    StaleCompletion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExportControllerState {
    Idle,
    Running {
        job_id: WorkbenchJobId,
        selection: SelectionId,
        evidence_generation: u64,
    },
    Succeeded(CommittedBundleSummary),
    Failed {
        request: ExportRequest,
        error: ExportControllerError,
    },
    Cancelling {
        job_id: WorkbenchJobId,
    },
}

pub(crate) struct ExportController {
    jobs: JobCoordinator,
    state: ExportControllerState,
    active_job: Option<JobHandle>,
    active_request: Option<ExportRequest>,
}

impl ExportController {
    pub(crate) fn new(worker_count: usize) -> Self {
        Self {
            jobs: JobCoordinator::new(worker_count),
            state: ExportControllerState::Idle,
            active_job: None,
            active_request: None,
        }
    }

    pub(crate) fn state(&self) -> &ExportControllerState {
        &self.state
    }

    pub(crate) fn submit<F>(
        &mut self,
        request: ExportRequest,
        worker: F,
    ) -> Result<WorkbenchJobId, ExportControllerError>
    where
        F: FnOnce(CancellationToken, ExportRequest) -> JobOutcome + Send + 'static,
    {
        if self.active_job.is_some() {
            return Err(ExportControllerError::Busy);
        }
        let worker_request = request.clone();
        let handle = self
            .jobs
            .submit_with_metadata(
                WorkbenchJobKind::Export,
                WorkbenchJobGeneration(request.evidence_generation),
                move |token| worker(token, worker_request),
            )
            .map_err(map_submit_error)?;
        let job_id = handle.id();
        self.active_request = Some(request.clone());
        self.active_job = Some(handle);
        self.state = ExportControllerState::Running {
            job_id,
            selection: request.selection,
            evidence_generation: request.evidence_generation,
        };
        Ok(job_id)
    }

    pub(crate) fn cancel(&mut self) -> Result<(), ExportControllerError> {
        let Some(job) = self.active_job.as_ref() else {
            return Ok(());
        };
        job.cancel();
        self.state = ExportControllerState::Cancelling { job_id: job.id() };
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> Option<Result<CommittedBundleSummary, ExportControllerError>> {
        let outcome = self.active_job.as_ref()?.outcome()?;
        let job = self.active_job.take()?;
        let request = self.active_request.take()?;
        let result = match outcome {
            JobOutcome::Succeeded => {
                let summary = CommittedBundleSummary {
                    selection: request.selection,
                    evidence_generation: request.evidence_generation,
                    bundle_label: request.bundle_label.clone(),
                };
                self.state = ExportControllerState::Succeeded(summary.clone());
                Ok(summary)
            }
            JobOutcome::Failed => self.fail(request, ExportControllerError::Failed),
            JobOutcome::Cancelled => self.fail(request, ExportControllerError::Cancelled),
            JobOutcome::Panicked => self.fail(request, ExportControllerError::Panicked),
        };
        drop(job);
        Some(result)
    }

    pub(crate) fn reject_stale_completion(
        &self,
        job_id: WorkbenchJobId,
        selection: SelectionId,
        evidence_generation: u64,
    ) -> Result<(), ExportControllerError> {
        match self.state {
            ExportControllerState::Running {
                job_id: active_job,
                selection: active_selection,
                evidence_generation: active_generation,
            } if job_id == active_job
                && selection == active_selection
                && evidence_generation == active_generation =>
            {
                Ok(())
            }
            _ => Err(ExportControllerError::StaleCompletion),
        }
    }

    pub(crate) fn shutdown(&mut self) {
        self.jobs.shutdown();
        self.active_job = None;
        self.active_request = None;
        self.state = ExportControllerState::Idle;
    }

    fn fail(
        &mut self,
        request: ExportRequest,
        error: ExportControllerError,
    ) -> Result<CommittedBundleSummary, ExportControllerError> {
        self.state = ExportControllerState::Failed {
            request,
            error: error.clone(),
        };
        Err(error)
    }
}

impl Drop for ExportController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn map_submit_error(error: JobSubmitError) -> ExportControllerError {
    match error {
        JobSubmitError::QueueFull => ExportControllerError::QueueFull,
        JobSubmitError::ShuttingDown => ExportControllerError::ShuttingDown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn request(generation: u64) -> ExportRequest {
        ExportRequest {
            selection: SelectionId(4),
            evidence_generation: generation,
            bundle_label: Arc::from("bundle"),
        }
    }

    fn wait_for_result(
        controller: &mut ExportController,
    ) -> Result<CommittedBundleSummary, ExportControllerError> {
        loop {
            if let Some(result) = controller.poll() {
                return result;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn successful_worker_commits_matching_summary() {
        let mut controller = ExportController::new(1);
        let request = request(8);
        let job_id = controller
            .submit(request.clone(), |_, _| JobOutcome::Succeeded)
            .unwrap();
        assert!(controller
            .reject_stale_completion(job_id, request.selection, request.evidence_generation)
            .is_ok());
        assert_eq!(
            wait_for_result(&mut controller),
            Ok(CommittedBundleSummary {
                selection: request.selection,
                evidence_generation: request.evidence_generation,
                bundle_label: request.bundle_label,
            })
        );
        assert!(matches!(
            controller.state(),
            ExportControllerState::Succeeded(_)
        ));
    }

    #[test]
    fn second_export_is_rejected_until_first_reaches_terminal_state() {
        let mut controller = ExportController::new(1);
        controller
            .submit(request(1), |token, _| {
                while !token.is_cancelled() {
                    thread::yield_now();
                }
                JobOutcome::Cancelled
            })
            .unwrap();
        assert_eq!(
            controller.submit(request(2), |_, _| JobOutcome::Succeeded),
            Err(ExportControllerError::Busy)
        );
        controller.cancel().unwrap();
        assert_eq!(
            wait_for_result(&mut controller),
            Err(ExportControllerError::Cancelled)
        );
    }

    #[test]
    fn stale_generation_cannot_be_accepted_as_current_completion() {
        let mut controller = ExportController::new(1);
        let request = request(3);
        let job_id = controller
            .submit(request.clone(), |_, _| JobOutcome::Succeeded)
            .unwrap();
        assert_eq!(
            controller.reject_stale_completion(job_id, request.selection, 99),
            Err(ExportControllerError::StaleCompletion)
        );
        let _ = wait_for_result(&mut controller);
    }
}
