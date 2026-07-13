//! Context-rich lifecycle and command failures for dataset showcases.

use std::{error::Error, fmt, path::PathBuf};

/// Result returned by showcase commands.
pub type Result<T> = std::result::Result<T, ShowcaseError>;

/// Error returned by showcase commands.
pub enum ShowcaseError {
    InvalidCommand {
        supplied: Option<String>,
    },
    InvalidManifestDirectory {
        path: PathBuf,
    },
    NotImplemented {
        dataset_id: &'static str,
        operation: &'static str,
    },
    Workflow {
        dataset_id: &'static str,
        operation: &'static str,
        source: Box<dyn Error + Send + Sync>,
    },
    InvalidArtifact {
        dataset_id: &'static str,
        path: PathBuf,
        reason: String,
    },
}

impl fmt::Display for ShowcaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand { supplied: Some(command) } => write!(
                formatter,
                "unsupported showcase command '{command}'; expected info, fetch, transform, analyse, visualise, or run"
            ),
            Self::InvalidCommand { supplied: None } => formatter.write_str(
                "missing showcase command; expected info, fetch, transform, analyse, visualise, or run",
            ),
            Self::InvalidManifestDirectory { path } => write!(
                formatter,
                "showcase manifest directory '{}' is not nested beneath the repository showcases directory",
                path.display()
            ),
            Self::NotImplemented {
                dataset_id,
                operation,
            } => write!(
                formatter,
                "{dataset_id} showcase command '{operation}' is not implemented; external dataset acquisition and processing have not been enabled"
            ),
            Self::Workflow {
                dataset_id,
                operation,
                source,
            } => write!(
                formatter,
                "{dataset_id} showcase failed to {operation}: {source}"
            ),
            Self::InvalidArtifact {
                dataset_id,
                path,
                reason,
            } => write!(
                formatter,
                "{dataset_id} showcase artifact '{}' is invalid: {reason}",
                path.display()
            ),
        }
    }
}

impl fmt::Debug for ShowcaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for ShowcaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Workflow { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl ShowcaseError {
    pub fn workflow(
        dataset_id: &'static str,
        operation: &'static str,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::Workflow {
            dataset_id,
            operation,
            source: Box::new(source),
        }
    }
}

/// Returns the standard scaffold error for a disabled lifecycle operation.
pub const fn not_implemented(dataset_id: &'static str, operation: &'static str) -> ShowcaseError {
    ShowcaseError::NotImplemented {
        dataset_id,
        operation,
    }
}
