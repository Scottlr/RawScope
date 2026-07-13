//! Intentional lifecycle and command failures for scaffolded showcases.

use std::{error::Error, fmt, path::PathBuf};

/// Result returned by showcase commands.
pub type Result<T> = std::result::Result<T, ShowcaseError>;

/// Error returned by scaffolded showcase commands.
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
        }
    }
}

impl fmt::Debug for ShowcaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for ShowcaseError {}

/// Returns the standard scaffold error for a disabled lifecycle operation.
pub const fn not_implemented(dataset_id: &'static str, operation: &'static str) -> ShowcaseError {
    ShowcaseError::NotImplemented {
        dataset_id,
        operation,
    }
}
