//! Typed workbench operation failures and recovery classification.

use std::{error::Error, fmt, io};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryClass {
    Retryable,
    ReloadDataset,
    RecreateGpu,
    Fatal,
}

#[derive(Debug)]
pub(crate) enum JobError {
    QueueFull,
    ShuttingDown,
    Cancelled,
    Panicked,
    Failed,
}

impl fmt::Display for JobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::QueueFull => "workbench job queue is full",
            Self::ShuttingDown => "workbench job coordinator is shutting down",
            Self::Cancelled => "workbench job was cancelled",
            Self::Panicked => "workbench job panicked",
            Self::Failed => "workbench job failed",
        };
        formatter.write_str(message)
    }
}

impl Error for JobError {}

impl From<crate::job_coordinator::JobSubmitError> for JobError {
    fn from(error: crate::job_coordinator::JobSubmitError) -> Self {
        match error {
            crate::job_coordinator::JobSubmitError::QueueFull => Self::QueueFull,
            crate::job_coordinator::JobSubmitError::ShuttingDown => Self::ShuttingDown,
        }
    }
}

#[derive(Debug)]
pub(crate) enum WorkbenchOperationError {
    Session(io::Error),
    Job(JobError),
}

impl WorkbenchOperationError {
    pub(crate) const fn recovery_class(&self) -> RecoveryClass {
        match self {
            Self::Session(_) => RecoveryClass::ReloadDataset,
            Self::Job(JobError::QueueFull | JobError::Cancelled) => RecoveryClass::Retryable,
            Self::Job(JobError::ShuttingDown | JobError::Panicked | JobError::Failed) => {
                RecoveryClass::Fatal
            }
        }
    }
}

impl fmt::Display for WorkbenchOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(error) => write!(formatter, "session operation failed: {error}"),
            Self::Job(error) => write!(formatter, "job operation failed: {error}"),
        }
    }
}

impl Error for WorkbenchOperationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Session(error) => Some(error),
            Self::Job(error) => Some(error),
        }
    }
}

impl From<io::Error> for WorkbenchOperationError {
    fn from(error: io::Error) -> Self {
        Self::Session(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_error_preserves_source_and_reload_classification() {
        let source = io::Error::new(io::ErrorKind::NotFound, "manifest");
        let error = WorkbenchOperationError::Session(source);
        assert_eq!(error.recovery_class(), RecoveryClass::ReloadDataset);
        assert!(error.source().is_some());
    }

    #[test]
    fn queue_pressure_and_cancel_are_retryable() {
        assert_eq!(
            WorkbenchOperationError::Job(JobError::QueueFull).recovery_class(),
            RecoveryClass::Retryable
        );
        assert_eq!(
            WorkbenchOperationError::Job(JobError::Cancelled).recovery_class(),
            RecoveryClass::Retryable
        );
    }

    #[test]
    fn panic_and_failure_are_fatal() {
        assert_eq!(
            WorkbenchOperationError::Job(JobError::Panicked).recovery_class(),
            RecoveryClass::Fatal
        );
        assert_eq!(
            WorkbenchOperationError::Job(JobError::Failed).recovery_class(),
            RecoveryClass::Fatal
        );
    }
}
