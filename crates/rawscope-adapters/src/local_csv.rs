//! Public adapter for preparing RawScope sessions over existing local CSV files.

use std::{fs, io, path::Path, path::PathBuf};

use rawscope_session::{
    session_manifest_json, RawScopeSessionManifestV1, SessionDataFormat, SessionDatasetV1,
    SessionManifestError, SessionViewV1, RAWSCOPE_SESSION_ARTIFACT_KIND,
    RAWSCOPE_SESSION_SCHEMA_VERSION,
};
use thiserror::Error;

use crate::PreparedAdapterSession;

/// File name used for sessions prepared over existing local datasets.
pub const LOCAL_SESSION_MANIFEST_FILE_NAME: &str = "analysis.rawscope.json";

/// Builder for a scatter or timeline session backed by an existing local CSV file.
///
/// The prepared bundle contains only a versioned RawScope manifest. The source
/// CSV remains in place and is referenced by its canonical path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCsvSession {
    dataset_path: PathBuf,
    view: LocalCsvView,
    profile: Option<String>,
    display_name: Option<String>,
    evidence_key: Option<String>,
    limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalCsvView {
    Scatter { x: String, y: String },
    Timeline { time: String, lane: String },
}

impl LocalCsvSession {
    /// Starts a local CSV scatter-session definition.
    #[must_use]
    pub fn scatter(
        dataset_path: impl Into<PathBuf>,
        x_column: impl Into<String>,
        y_column: impl Into<String>,
    ) -> Self {
        Self {
            dataset_path: dataset_path.into(),
            view: LocalCsvView::Scatter {
                x: x_column.into(),
                y: y_column.into(),
            },
            profile: None,
            display_name: None,
            evidence_key: None,
            limit: None,
        }
    }

    /// Starts a local CSV timeline-session definition.
    #[must_use]
    pub fn timeline(
        dataset_path: impl Into<PathBuf>,
        time_column: impl Into<String>,
        lane_column: impl Into<String>,
    ) -> Self {
        Self {
            dataset_path: dataset_path.into(),
            view: LocalCsvView::Timeline {
                time: time_column.into(),
                lane: lane_column.into(),
            },
            profile: None,
            display_name: None,
            evidence_key: None,
            limit: None,
        }
    }

    /// Selects a RawScope dataset profile for domain-aware defaults.
    #[must_use]
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Adds a human-readable dataset name to the prepared session.
    #[must_use]
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Identifies the source column retained as row-level evidence.
    #[must_use]
    pub fn evidence_key(mut self, evidence_key: impl Into<String>) -> Self {
        self.evidence_key = Some(evidence_key.into());
        self
    }

    /// Caps the number of source rows loaded by RawScope.
    #[must_use]
    pub const fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Writes and validates a persistent RawScope session manifest.
    pub fn prepare(
        self,
        destination: impl AsRef<Path>,
    ) -> Result<PreparedAdapterSession, LocalCsvSessionError> {
        let destination = destination.as_ref();
        if destination.exists() {
            return Err(LocalCsvSessionError::DestinationExists {
                path: destination.to_path_buf(),
            });
        }

        let dataset_path =
            fs::canonicalize(&self.dataset_path).map_err(|source| LocalCsvSessionError::Io {
                operation: "resolve local CSV dataset",
                path: self.dataset_path.clone(),
                source,
            })?;
        fs::create_dir_all(destination).map_err(|source| LocalCsvSessionError::Io {
            operation: "create RawScope session directory",
            path: destination.to_path_buf(),
            source,
        })?;

        let manifest_path = destination.join(LOCAL_SESSION_MANIFEST_FILE_NAME);
        let preparation = write_manifest(&manifest_path, dataset_path, self)
            .and_then(|()| PreparedAdapterSession::from_manifest(&manifest_path, None));

        match preparation {
            Ok(prepared) => Ok(prepared),
            Err(error) => {
                fs::remove_dir_all(destination).map_err(|source| {
                    LocalCsvSessionError::Cleanup {
                        path: destination.to_path_buf(),
                        original: error.to_string(),
                        source,
                    }
                })?;
                Err(error.into())
            }
        }
    }
}

fn write_manifest(
    manifest_path: &Path,
    dataset_path: PathBuf,
    session: LocalCsvSession,
) -> Result<(), SessionManifestError> {
    let manifest = RawScopeSessionManifestV1 {
        artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
        schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
        dataset: SessionDatasetV1 {
            path: dataset_path,
            format: SessionDataFormat::Csv,
            display_name: session.display_name,
            limit: session.limit,
            evidence_key: session.evidence_key,
        },
        view: match session.view {
            LocalCsvView::Scatter { x, y } => SessionViewV1::Scatter {
                x,
                y,
                profile: session.profile,
            },
            LocalCsvView::Timeline { time, lane } => SessionViewV1::Timeline {
                time,
                lane,
                profile: session.profile,
            },
        },
    };
    let json = session_manifest_json(&manifest)?;
    fs::write(manifest_path, json).map_err(|source| SessionManifestError::Io {
        path: manifest_path.to_path_buf(),
        source,
    })
}

/// Failure to prepare a RawScope session over an existing local CSV file.
#[derive(Debug, Error)]
pub enum LocalCsvSessionError {
    /// The destination is caller-owned and is never overwritten.
    #[error("RawScope local CSV session destination already exists: '{}'", path.display())]
    DestinationExists { path: PathBuf },

    /// Filesystem access failed with operation and path context.
    #[error("failed to {operation} '{}': {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The generated manifest failed the shared RawScope session contract.
    #[error(transparent)]
    Session(#[from] SessionManifestError),

    /// Cleanup failed after session preparation had already failed.
    #[error(
        "local CSV session preparation failed ({original}); cleanup also failed for '{}': {source}",
        path.display()
    )]
    Cleanup {
        path: PathBuf,
        original: String,
        #[source]
        source: io::Error,
    },
}
