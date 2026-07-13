//! Deterministic local paths for ignored showcase artifacts.

use std::path::{Path, PathBuf};

use crate::{Result, ShowcaseError};

/// Repository-relative root for data that must never be committed.
pub const SHOWCASE_DATA_DIRECTORY: &str = ".showcase-data";

/// Planned local directories for one dataset showcase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShowcasePaths {
    pub downloads: PathBuf,
    pub extracted: PathBuf,
    pub transformed: PathBuf,
    pub analysis: PathBuf,
    pub cache: PathBuf,
}

impl ShowcasePaths {
    fn new(repository_root: &Path, dataset_id: &str) -> Self {
        let data_root = repository_root.join(SHOWCASE_DATA_DIRECTORY);
        Self {
            downloads: data_root.join("downloads").join(dataset_id),
            extracted: data_root.join("extracted").join(dataset_id),
            transformed: data_root.join("transformed").join(dataset_id),
            analysis: data_root.join("analysis").join(dataset_id),
            cache: data_root.join("cache").join(dataset_id),
        }
    }
}

/// Repository and path context supplied to showcase lifecycle operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShowcaseContext {
    pub repository_root: PathBuf,
    pub paths: ShowcasePaths,
}

impl ShowcaseContext {
    /// Resolves the repository root from a `showcases/<dataset>` manifest directory.
    pub fn from_manifest_dir(manifest_dir: impl AsRef<Path>, dataset_id: &str) -> Result<Self> {
        let manifest_dir = manifest_dir.as_ref();
        let showcases_dir =
            manifest_dir
                .parent()
                .ok_or_else(|| ShowcaseError::InvalidManifestDirectory {
                    path: manifest_dir.to_path_buf(),
                })?;
        let repository_root =
            showcases_dir
                .parent()
                .ok_or_else(|| ShowcaseError::InvalidManifestDirectory {
                    path: manifest_dir.to_path_buf(),
                })?;

        Ok(Self {
            repository_root: repository_root.to_path_buf(),
            paths: ShowcasePaths::new(repository_root, dataset_id),
        })
    }
}
