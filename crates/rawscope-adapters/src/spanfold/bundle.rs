//! Transactional CSV and session-v1 bundle materialization for SpanFold intervals.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use rawscope_session::{
    load_session_manifest, session_manifest_json, RawScopeSessionManifestV1, SessionDataFormat,
    SessionDatasetV1, SessionViewV1, RAWSCOPE_SESSION_ARTIFACT_KIND,
    RAWSCOPE_SESSION_SCHEMA_VERSION,
};
use spanfold::ComparisonResult;

use crate::{PreparedAdapterSession, SessionAdapter};

use super::{
    error::SpanfoldAdapterError,
    interval::{SpanfoldIntervalDataset, SpanfoldIntervalTransform},
};

/// Session manifest file name shared with other RawScope preparation bridges.
pub const SPANFOLD_SESSION_MANIFEST_FILE_NAME: &str = "analysis.rawscope.json";

/// Materialized rectangular SpanFold comparison dataset file name.
pub const SPANFOLD_DATASET_FILE_NAME: &str = "spanfold-comparison.csv";

/// Authoritative SpanFold row-identity column used for RawScope evidence lookup.
pub const SPANFOLD_EVIDENCE_KEY_COLUMN: &str = "spanfold_row_id";

/// Default scatter x column, normalized relative to the earliest interval start.
pub const RAWSCOPE_SCATTER_START_COLUMN: &str = "start_offset";

/// Default scatter y column containing exact interval duration magnitudes.
pub const RAWSCOPE_SCATTER_DURATION_COLUMN: &str = "duration";

static NEXT_STAGING_ID: AtomicUsize = AtomicUsize::new(0);

impl SessionAdapter for SpanfoldIntervalTransform {
    type Input = ComparisonResult;
    type Error = SpanfoldAdapterError;

    fn prepare_session(
        &self,
        input: &Self::Input,
        destination: &Path,
    ) -> Result<PreparedAdapterSession, Self::Error> {
        self.transform(input)?.prepare_session(destination)
    }
}

impl SpanfoldIntervalTransform {
    /// Transforms a comparison and atomically prepares a persistent RawScope session bundle.
    pub fn prepare_session(
        &self,
        result: &ComparisonResult,
        destination: impl AsRef<Path>,
    ) -> Result<PreparedAdapterSession, SpanfoldAdapterError> {
        <Self as SessionAdapter>::prepare_session(self, result, destination.as_ref())
    }
}

impl SpanfoldIntervalDataset {
    /// Atomically materializes this transformed interval table and its session manifest.
    pub fn prepare_session(
        self,
        destination: impl AsRef<Path>,
    ) -> Result<PreparedAdapterSession, SpanfoldAdapterError> {
        prepare_session_bundle(&self, destination.as_ref())
    }
}

fn prepare_session_bundle(
    dataset: &SpanfoldIntervalDataset,
    destination: &Path,
) -> Result<PreparedAdapterSession, SpanfoldAdapterError> {
    if destination.exists() {
        return Err(SpanfoldAdapterError::DestinationExists {
            path: destination.to_path_buf(),
        });
    }

    let destination_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| SpanfoldAdapterError::InvalidDestination {
            path: destination.to_path_buf(),
        })?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| SpanfoldAdapterError::Io {
        operation: "create session parent directory",
        path: parent.to_path_buf(),
        source,
    })?;

    let staging_id = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
    let staging_name = format!(
        ".{destination_name}.rawscope-stage-{}-{staging_id}",
        std::process::id()
    );
    let staging_dir = parent.join(staging_name);
    fs::create_dir(&staging_dir).map_err(|source| SpanfoldAdapterError::Io {
        operation: "create exclusive session staging directory",
        path: staging_dir.clone(),
        source,
    })?;

    let preparation = write_staged_bundle(dataset, &staging_dir)
        .and_then(|_| publish_staged_bundle(&staging_dir, destination));
    if let Err(error) = preparation {
        return match fs::remove_dir_all(&staging_dir) {
            Ok(()) => Err(error),
            Err(source) => Err(SpanfoldAdapterError::Cleanup {
                path: staging_dir,
                original: error.to_string(),
                source,
            }),
        };
    }

    let manifest_path = destination.join(SPANFOLD_SESSION_MANIFEST_FILE_NAME);
    PreparedAdapterSession::from_manifest(manifest_path, Some(dataset.rows().len()))
        .map_err(Into::into)
}

fn write_staged_bundle(
    dataset: &SpanfoldIntervalDataset,
    staging_dir: &Path,
) -> Result<(), SpanfoldAdapterError> {
    let dataset_path = staging_dir.join(SPANFOLD_DATASET_FILE_NAME);
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_path(&dataset_path)
        .map_err(|source| SpanfoldAdapterError::Csv {
            path: dataset_path.clone(),
            source,
        })?;
    for row in dataset.rows() {
        writer
            .serialize(row)
            .map_err(|source| SpanfoldAdapterError::Csv {
                path: dataset_path.clone(),
                source,
            })?;
    }
    writer.flush().map_err(|source| SpanfoldAdapterError::Io {
        operation: "flush SpanFold CSV dataset",
        path: dataset_path,
        source,
    })?;

    let manifest = RawScopeSessionManifestV1 {
        artifact_kind: RAWSCOPE_SESSION_ARTIFACT_KIND.to_string(),
        schema_version: RAWSCOPE_SESSION_SCHEMA_VERSION,
        dataset: SessionDatasetV1 {
            path: PathBuf::from(SPANFOLD_DATASET_FILE_NAME),
            format: SessionDataFormat::Csv,
            display_name: Some(format!("SpanFold: {}", dataset.plan_name())),
            limit: None,
            evidence_key: Some(SPANFOLD_EVIDENCE_KEY_COLUMN.to_string()),
        },
        view: SessionViewV1::Scatter {
            x: RAWSCOPE_SCATTER_START_COLUMN.to_string(),
            y: RAWSCOPE_SCATTER_DURATION_COLUMN.to_string(),
            profile: None,
        },
    };
    let manifest_json = session_manifest_json(&manifest)?;
    let manifest_path = staging_dir.join(SPANFOLD_SESSION_MANIFEST_FILE_NAME);
    fs::write(&manifest_path, manifest_json).map_err(|source| SpanfoldAdapterError::Io {
        operation: "write RawScope session manifest",
        path: manifest_path.clone(),
        source,
    })?;

    load_session_manifest(&manifest_path)?;
    Ok(())
}

fn publish_staged_bundle(
    staging_dir: &Path,
    destination: &Path,
) -> Result<(), SpanfoldAdapterError> {
    fs::rename(staging_dir, destination).map_err(|source| SpanfoldAdapterError::Io {
        operation: "publish complete SpanFold session bundle",
        path: destination.to_path_buf(),
        source,
    })
}
