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

const JOB_QUEUE_CAPACITY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorkbenchJobId(u64);

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
    cancel: CancellationToken,
    terminal: Arc<Mutex<Option<JobOutcome>>>,
}

impl JobHandle {
    pub(crate) fn id(&self) -> WorkbenchJobId {
        self.id
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
    job: Job,
    cancel: CancellationToken,
    terminal: Arc<Mutex<Option<JobOutcome>>>,
}

pub(crate) struct JobCoordinator {
    sender: Option<SyncSender<QueuedJob>>,
    workers: Vec<JoinHandle<()>>,
    next_id: AtomicU64,
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
        }
    }

    pub(crate) fn submit<F>(&self, job: F) -> Result<JobHandle, JobSubmitError>
    where
        F: FnOnce(CancellationToken) -> JobOutcome + Send + 'static,
    {
        let id = WorkbenchJobId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let cancel = CancellationToken(Arc::new(AtomicBool::new(false)));
        let terminal = Arc::new(Mutex::new(None));
        let queued = QueuedJob {
            job: Box::new(job),
            cancel: cancel.clone(),
            terminal: Arc::clone(&terminal),
        };
        let sender = self.sender.as_ref().ok_or(JobSubmitError::ShuttingDown)?;
        match sender.try_send(queued) {
            Ok(()) => Ok(JobHandle {
                id,
                cancel,
                terminal,
            }),
            Err(TrySendError::Full(_)) => Err(JobSubmitError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(JobSubmitError::ShuttingDown),
        }
    }
}

impl Drop for JobCoordinator {
    fn drop(&mut self) {
        self.sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
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
        })
        .expect("job worker thread should spawn")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_jobs_reach_durable_terminal_state() {
        let coordinator = JobCoordinator::new(1);
        let handle = coordinator.submit(|_| JobOutcome::Succeeded).unwrap();
        while handle.outcome().is_none() {
            std::thread::yield_now();
        }
        assert_eq!(handle.outcome(), Some(JobOutcome::Succeeded));
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
}
