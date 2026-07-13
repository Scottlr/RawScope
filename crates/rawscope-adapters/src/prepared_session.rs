//! Shared handle for sessions prepared by source adapters.

use std::{
    path::{Path, PathBuf},
    process::Child,
};

use rawscope_session::{
    load_session_manifest, SessionManifestError, WorkbenchLaunchError, WorkbenchLauncher,
};

/// Paths and metadata shared by every successfully prepared adapter session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedAdapterSession {
    bundle_dir: PathBuf,
    dataset_path: PathBuf,
    manifest_path: PathBuf,
    row_count: usize,
}

impl PreparedAdapterSession {
    /// Validates an existing session manifest and returns its resolved adapter handle.
    ///
    /// Adapter implementations should call this only after their dataset and
    /// manifest have been completely materialized.
    pub fn from_manifest(
        manifest_path: impl AsRef<Path>,
        row_count: usize,
    ) -> Result<Self, SessionManifestError> {
        let session = load_session_manifest(manifest_path)?;
        let bundle_dir = session
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        Ok(Self {
            bundle_dir,
            dataset_path: session.dataset.path,
            manifest_path: session.manifest_path,
            row_count,
        })
    }

    /// Persistent bundle directory selected by the caller.
    #[must_use]
    pub fn bundle_dir(&self) -> &Path {
        &self.bundle_dir
    }

    /// Materialized dataset path.
    #[must_use]
    pub fn dataset_path(&self) -> &Path {
        &self.dataset_path
    }

    /// Versioned RawScope session manifest path.
    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// Number of source rows materialized by the adapter.
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    /// Starts the native workbench using environment and `PATH` resolution.
    pub fn launch(&self) -> Result<Child, WorkbenchLaunchError> {
        self.launch_with(&WorkbenchLauncher::default())
    }

    /// Starts the native workbench using an explicitly configured launcher.
    pub fn launch_with(&self, launcher: &WorkbenchLauncher) -> Result<Child, WorkbenchLaunchError> {
        launcher.launch(&self.manifest_path)
    }
}
