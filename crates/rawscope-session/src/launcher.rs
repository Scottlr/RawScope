//! Native workbench process launching for validated local sessions.

use std::{
    env,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{Child, Command},
};

use crate::{load_session_manifest, SessionManifestError};

/// Environment variable used to override the native workbench executable path.
pub const RAWSCOPE_WORKBENCH_ENVIRONMENT_VARIABLE: &str = "RAWSCOPE_WORKBENCH";

/// Native workbench command resolved through `PATH` when no path is configured.
pub const RAWSCOPE_WORKBENCH_COMMAND: &str = "rawscope-workbench";

/// Resolves and starts the native workbench for one validated session manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkbenchLauncher {
    explicit_executable: Option<PathBuf>,
}

impl WorkbenchLauncher {
    /// Creates a launcher that uses the environment override and then `PATH`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            explicit_executable: None,
        }
    }

    /// Uses one explicit native executable path instead of environment or `PATH` resolution.
    #[must_use]
    pub fn with_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.explicit_executable = Some(executable.into());
        self
    }

    /// Validates the session and starts the native workbench with `--session`.
    pub fn launch(&self, manifest_path: impl AsRef<Path>) -> Result<Child, WorkbenchLaunchError> {
        let session =
            load_session_manifest(manifest_path).map_err(WorkbenchLaunchError::Session)?;
        let executable = self.resolve_executable()?;
        Command::new(&executable)
            .arg("--session")
            .arg(&session.manifest_path)
            .spawn()
            .map_err(|source| WorkbenchLaunchError::Spawn {
                executable,
                manifest_path: session.manifest_path,
                source,
            })
    }

    fn resolve_executable(&self) -> Result<PathBuf, WorkbenchLaunchError> {
        if let Some(executable) = &self.explicit_executable {
            validate_executable_path(executable)?;
            return Ok(executable.clone());
        }

        if let Some(executable) = env::var_os(RAWSCOPE_WORKBENCH_ENVIRONMENT_VARIABLE) {
            let executable = PathBuf::from(executable);
            validate_executable_path(&executable)?;
            return Ok(executable);
        }

        Ok(PathBuf::from(RAWSCOPE_WORKBENCH_COMMAND))
    }
}

fn validate_executable_path(path: &Path) -> Result<(), WorkbenchLaunchError> {
    let metadata =
        fs::metadata(path).map_err(|source| WorkbenchLaunchError::ExecutableMetadata {
            path: path.to_path_buf(),
            source,
        })?;
    if !metadata.is_file() {
        return Err(WorkbenchLaunchError::ExecutableNotRegularFile {
            path: path.to_path_buf(),
        });
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        const EXECUTABLE_PERMISSION_BITS: u32 = 0o111;
        let is_executable = metadata.permissions().mode() & EXECUTABLE_PERMISSION_BITS != 0;
        if !is_executable {
            return Err(WorkbenchLaunchError::ExecutablePermission {
                path: path.to_path_buf(),
            });
        }
    }

    Ok(())
}

/// Failure to validate a session, resolve its executable, or start the workbench.
#[derive(Debug)]
pub enum WorkbenchLaunchError {
    /// The requested session manifest or referenced dataset is invalid.
    Session(SessionManifestError),
    /// An explicitly configured executable path could not be inspected.
    ExecutableMetadata { path: PathBuf, source: io::Error },
    /// An explicitly configured executable path is not a regular file.
    ExecutableNotRegularFile { path: PathBuf },
    /// An explicitly configured executable lacks an executable permission bit.
    #[cfg(unix)]
    ExecutablePermission { path: PathBuf },
    /// The operating system rejected process creation.
    Spawn {
        executable: PathBuf,
        manifest_path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for WorkbenchLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(source) => {
                write!(formatter, "RawScope session is not launchable: {source}")
            }
            Self::ExecutableMetadata { path, source } => write!(
                formatter,
                "cannot inspect RawScope workbench executable '{}': {source}",
                path.display()
            ),
            Self::ExecutableNotRegularFile { path } => write!(
                formatter,
                "RawScope workbench executable '{}' is not a regular file",
                path.display()
            ),
            #[cfg(unix)]
            Self::ExecutablePermission { path } => write!(
                formatter,
                "RawScope workbench executable '{}' is not executable",
                path.display()
            ),
            Self::Spawn {
                executable,
                manifest_path,
                source,
            } => write!(
                formatter,
                "failed to start RawScope workbench '{}' for session '{}': {source}",
                executable.display(),
                manifest_path.display()
            ),
        }
    }
}

impl Error for WorkbenchLaunchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Session(source) => Some(source),
            Self::ExecutableMetadata { source, .. } | Self::Spawn { source, .. } => Some(source),
            Self::ExecutableNotRegularFile { .. } => None,
            #[cfg(unix)]
            Self::ExecutablePermission { .. } => None,
        }
    }
}
