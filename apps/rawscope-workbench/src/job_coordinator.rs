//! Bounded, joined native jobs for workbench-owned background IO.

use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use winit::event_loop::EventLoopProxy;
use tracing::debug;

use crate::workbench_event::WorkbenchUserEvent;

const JOB_QUEUE_CAPACITY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorkbenchJobId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorkbenchJobGeneration(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WorkbenchJobKind {
    Generic,
    StartupResolution,
    GpuInitialization,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Panicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobSubmitError {
    QueueFull,
    ShuttingDown,
}

#[derive(Debug, Clone)]
pub(crate) struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(crate) struct JobHandle {
    id: WorkbenchJobId,
    kind: WorkbenchJobKind,
    generation: WorkbenchJobGeneration,
    cancel: CancellationToken,
    terminal: Arc<Mutex<Option<JobOutcome>>>,
}

impl JobHandle {
    pub(crate) fn id(&self) -> WorkbenchJobId {
        self.id
    }
    pub(crate) fn kind(&self) -> WorkbenchJobKind {
        self.kind
    }
    pub(crate) fn generation(&self) -> WorkbenchJobGeneration {
        self.generation
    }
    pub(crate) fn cancel(&self) {
        self.cancel.cancel();
    }
    pub(crate) fn outcome(&self) -> Option<JobOutcome> {
        *self.terminal.lock().expect("job terminal lock")
    }
}

type Job = Box<dyn FnOnce(CancellationToken) -> JobOutcome + Send + 'static>;

struct QueuedJob {
    id: WorkbenchJobId,
    kind: WorkbenchJobKind,
    generation: WorkbenchJobGeneration,
    job: Job,
    cancel: CancellationToken,
    terminal: Arc<Mutex<Option<JobOutcome>>>,
    proxy: Option<EventLoopProxy<WorkbenchUserEvent>>,
}

pub(crate) struct JobCoordinator {
    sender: Option<SyncSender<QueuedJob>>,
    workers: Vec<JoinHandle<()>>,
    next_id: AtomicU64,
    cancellations: Arc<Mutex<Vec<CancellationToken>>>,
    proxy: Option<EventLoopProxy<WorkbenchUserEvent>>,
}

impl JobCoordinator {
    pub(crate) fn new(worker_count: usize) -> Self {
        let (sender, receiver) = sync_channel(JOB_QUEUE_CAPACITY);
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..worker_count.max(1))
            .map(|index| spawn_worker(index, Arc::clone(&receiver)))
            .collect();
        Self {
            sender: Some(sender),
            workers,
            next_id: AtomicU64::new(1),
            cancellations: Arc::new(Mutex::new(Vec::new())),
            proxy: None,
        }
    }

    pub(crate) fn with_proxy(
        worker_count: usize,
        proxy: EventLoopProxy<WorkbenchUserEvent>,
    ) -> Self {
        let mut coordinator = Self::new(worker_count);
        coordinator.proxy = Some(proxy);
        coordinator
    }

    pub(crate) fn submit<F>(&self, job: F) -> Result<JobHandle, JobSubmitError>
    where
        F: FnOnce(CancellationToken) -> JobOutcome + Send + 'static,
    {
        self.submit_with_metadata(WorkbenchJobKind::Generic, WorkbenchJobGeneration(0), job)
    }

    pub(crate) fn submit_with_metadata<F>(
        &self,
        kind: WorkbenchJobKind,
        generation: WorkbenchJobGeneration,
        job: F,
    ) -> Result<JobHandle, JobSubmitError>
    where
        F: FnOnce(CancellationToken) -> JobOutcome + Send + 'static,
    {
        let id = WorkbenchJobId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let cancel = CancellationToken(Arc::new(AtomicBool::new(false)));
        let terminal = Arc::new(Mutex::new(None));
        let queued = QueuedJob {
            id,
            kind,
            generation,
            job: Box::new(job),
            cancel: cancel.clone(),
            terminal: Arc::clone(&terminal),
            proxy: self.proxy.clone(),
        };
        let sender = self.sender.as_ref().ok_or(JobSubmitError::ShuttingDown)?;
        match sender.try_send(queued) {
            Ok(()) => {
                self.cancellations
                    .lock()
                    .expect("job cancellation lock")
                    .push(cancel.clone());
                Ok(JobHandle {
                    id,
                    kind,
                    generation,
                    cancel,
                    terminal,
                })
            }
            Err(TrySendError::Full(_)) => Err(JobSubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(JobSubmitError::ShuttingDown),
        }
    }

    /// Stops admissions and asks every accepted job to finish cooperatively.
    /// Queued jobs still reach a terminal cancellation outcome on the workers.
    pub(crate) fn shutdown(&mut self) {
        self.sender.take();
        for cancellation in self
            .cancellations
            .lock()
            .expect("job cancellation lock")
            .iter()
        {
            cancellation.cancel();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl Drop for JobCoordinator {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_worker(index: usize, receiver: Arc<Mutex<Receiver<QueuedJob>>>) -> JoinHandle<()> {
    thread::Builder::new()
        .name(format!("rawscope-job-{index}"))
        .spawn(move || loop {
            let queued = receiver.lock().expect("job receiver lock").recv();
            let Ok(queued) = queued else {
                break;
            };
            let outcome = if queued.cancel.is_cancelled() {
                JobOutcome::Cancelled
            } else {
                catch_unwind(AssertUnwindSafe(|| (queued.job)(queued.cancel.clone())))
                    .unwrap_or(JobOutcome::Panicked)
            };
            *queued.terminal.lock().expect("job terminal lock") = Some(outcome);
            debug!(
                job_id = queued.id.0,
                job_kind = ?queued.kind,
                generation = queued.generation.0,
                outcome = ?outcome,
                "workbench job reached terminal state"
            );
            if let Some(proxy) = queued.proxy {
                let _ = proxy.send_event(WorkbenchUserEvent::JobCompleted {
                    job_id: queued.id,
                    outcome,
                });
            }
        })
        .expect("job worker thread should spawn")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn accepted_jobs_reach_durable_terminal_state() {
        let coordinator = JobCoordinator::new(1);
        let handle = coordinator.submit(|_| JobOutcome::Succeeded).unwrap();
        assert_eq!(handle.kind(), WorkbenchJobKind::Generic);
        assert_eq!(handle.generation(), WorkbenchJobGeneration(0));
        while handle.outcome().is_none() {
            std::thread::yield_now();
        }
        assert_eq!(handle.outcome(), Some(JobOutcome::Succeeded));
    }

    #[test]
    fn accepted_jobs_retain_typed_kind_and_generation_metadata() {
        let coordinator = JobCoordinator::new(1);
        let handle = coordinator
            .submit_with_metadata(
                WorkbenchJobKind::StartupResolution,
                WorkbenchJobGeneration(7),
                |_| JobOutcome::Succeeded,
            )
            .unwrap();
        while handle.outcome().is_none() {
            std::thread::yield_now();
        }
        assert_eq!(handle.kind(), WorkbenchJobKind::StartupResolution);
        assert_eq!(handle.generation(), WorkbenchJobGeneration(7));
    }

    #[test]
    fn cancellation_and_panic_are_terminal_outcomes() {
        let coordinator = JobCoordinator::new(1);
        let cancelled = coordinator
            .submit(|token| {
                token.cancel();
                JobOutcome::Cancelled
            })
            .unwrap();
        let panicked = coordinator.submit(|_| panic!("test panic")).unwrap();
        while cancelled.outcome().is_none() || panicked.outcome().is_none() {
            std::thread::yield_now();
        }
        assert_eq!(cancelled.outcome(), Some(JobOutcome::Cancelled));
        assert_eq!(panicked.outcome(), Some(JobOutcome::Panicked));
    }

    #[test]
    fn shutdown_cancels_a_running_cooperative_job_before_joining() {
        let mut coordinator = JobCoordinator::new(1);
        let (started_sender, started_receiver) = mpsc::channel();
        let handle = coordinator
            .submit(move |token| {
                started_sender.send(()).unwrap();
                while !token.is_cancelled() {
                    std::thread::yield_now();
                }
                JobOutcome::Cancelled
            })
            .unwrap();
        started_receiver.recv().unwrap();
        coordinator.shutdown();
        assert_eq!(handle.outcome(), Some(JobOutcome::Cancelled));
        assert!(coordinator.submit(|_| JobOutcome::Succeeded).is_err());
    }
}
