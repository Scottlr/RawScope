//! Bounded GPU-context initialization for the workbench startup path.

use std::{
    io,
    sync::{Arc, Mutex},
};

use rawscope_gpu::GpuContext;
use winit::window::Window;

use crate::job_coordinator::{
    CancellationToken, JobCoordinator, JobHandle, JobOutcome, JobSubmitError,
};

#[derive(Clone)]
pub(crate) struct GpuResolution {
    result: Arc<Mutex<Option<Result<GpuContext, io::Error>>>>,
}

impl GpuResolution {
    pub(crate) fn take(&self) -> Option<Result<GpuContext, io::Error>> {
        self.result.lock().expect("GPU startup result lock").take()
    }
}

pub(crate) fn submit_gpu_initialization(
    coordinator: &JobCoordinator,
    window: Arc<Window>,
) -> Result<(JobHandle, GpuResolution), JobSubmitError> {
    let result = Arc::new(Mutex::new(None));
    let worker_result = Arc::clone(&result);
    let handle = coordinator.submit(move |token: CancellationToken| {
        if token.is_cancelled() {
            return JobOutcome::Cancelled;
        }
        let resolved = pollster::block_on(GpuContext::new(window))
            .map_err(|error| io::Error::other(error.to_string()));
        let outcome = if resolved.is_ok() {
            JobOutcome::Succeeded
        } else {
            JobOutcome::Failed
        };
        *worker_result.lock().expect("GPU startup result lock") = Some(resolved);
        outcome
    })?;
    Ok((handle, GpuResolution { result }))
}
