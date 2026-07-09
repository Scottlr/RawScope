//! Shared metadata and dataset identity for synthetic and local datasets.

use std::path::PathBuf;

const SYNTHETIC_POINTS_GENERATOR: &str = "synthetic-points";
const SYNTHETIC_EVENTS_GENERATOR: &str = "synthetic-events";
const SYNTHETIC_X_FIELD: &str = "synthetic_x";
const SYNTHETIC_Y_FIELD: &str = "synthetic_y";
const SYNTHETIC_TIME_FIELD: &str = "synthetic_timestamp";
const SYNTHETIC_LANE_FIELD: &str = "synthetic_lane";

/// Metadata describing how a synthetic dataset was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticDatasetMetadata {
    pub seed: u64,
    pub row_count: usize,
}

impl SyntheticDatasetMetadata {
    /// Creates metadata for a deterministic synthetic dataset.
    pub fn new(seed: u64, row_count: usize) -> Self {
        Self { seed, row_count }
    }
}

/// The high-level visual dataset shape supported by the current workbench.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualDatasetKind {
    Scatter,
    Timeline,
}

/// The source of a dataset shown in the workbench.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetSource {
    Synthetic { seed: u64, generator: &'static str },
    LocalCsv { path: PathBuf, limit: Option<usize> },
    LocalParquet { path: PathBuf, limit: Option<usize> },
}

/// The role a field binding plays in a visual dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetFieldRole {
    X,
    Y,
    Time,
    Lane,
}

/// A user-visible dataset field bound into a current visual workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetFieldBinding {
    pub role: DatasetFieldRole,
    pub column_name: String,
}

impl DatasetFieldBinding {
    /// Creates one dataset field binding.
    pub fn new(role: DatasetFieldRole, column_name: impl Into<String>) -> Self {
        Self {
            role,
            column_name: column_name.into(),
        }
    }
}

/// Identity and provenance for a dataset currently shown in RawScope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetIdentity {
    pub visual_kind: VisualDatasetKind,
    pub source: DatasetSource,
    pub row_count: usize,
    pub field_bindings: Vec<DatasetFieldBinding>,
    pub lane_labels: Vec<String>,
}

impl DatasetIdentity {
    /// Identity for a deterministic synthetic scatter dataset.
    pub fn synthetic_scatter(seed: u64, row_count: usize) -> Self {
        Self {
            visual_kind: VisualDatasetKind::Scatter,
            source: DatasetSource::Synthetic {
                seed,
                generator: SYNTHETIC_POINTS_GENERATOR,
            },
            row_count,
            field_bindings: vec![
                DatasetFieldBinding::new(DatasetFieldRole::X, SYNTHETIC_X_FIELD),
                DatasetFieldBinding::new(DatasetFieldRole::Y, SYNTHETIC_Y_FIELD),
            ],
            lane_labels: Vec::new(),
        }
    }

    /// Identity for a deterministic synthetic timeline dataset.
    pub fn synthetic_timeline(seed: u64, row_count: usize, lane_count: u32) -> Self {
        let lane_labels = (0..lane_count).map(|lane| format!("lane-{lane}")).collect();
        Self {
            visual_kind: VisualDatasetKind::Timeline,
            source: DatasetSource::Synthetic {
                seed,
                generator: SYNTHETIC_EVENTS_GENERATOR,
            },
            row_count,
            field_bindings: vec![
                DatasetFieldBinding::new(DatasetFieldRole::Time, SYNTHETIC_TIME_FIELD),
                DatasetFieldBinding::new(DatasetFieldRole::Lane, SYNTHETIC_LANE_FIELD),
            ],
            lane_labels,
        }
    }

    /// Identity for a local CSV scatter dataset.
    pub fn local_csv_scatter(
        path: PathBuf,
        row_count: usize,
        limit: Option<usize>,
        x_column: impl Into<String>,
        y_column: impl Into<String>,
    ) -> Self {
        Self {
            visual_kind: VisualDatasetKind::Scatter,
            source: DatasetSource::LocalCsv { path, limit },
            row_count,
            field_bindings: vec![
                DatasetFieldBinding::new(DatasetFieldRole::X, x_column),
                DatasetFieldBinding::new(DatasetFieldRole::Y, y_column),
            ],
            lane_labels: Vec::new(),
        }
    }

    /// Identity for a local CSV timeline dataset.
    pub fn local_csv_timeline(
        path: PathBuf,
        row_count: usize,
        limit: Option<usize>,
        time_column: impl Into<String>,
        lane_column: impl Into<String>,
        lane_labels: Vec<String>,
    ) -> Self {
        Self {
            visual_kind: VisualDatasetKind::Timeline,
            source: DatasetSource::LocalCsv { path, limit },
            row_count,
            field_bindings: vec![
                DatasetFieldBinding::new(DatasetFieldRole::Time, time_column),
                DatasetFieldBinding::new(DatasetFieldRole::Lane, lane_column),
            ],
            lane_labels,
        }
    }

    /// Identity for a local Parquet scatter dataset.
    pub fn local_parquet_scatter(
        path: PathBuf,
        row_count: usize,
        limit: Option<usize>,
        x_column: impl Into<String>,
        y_column: impl Into<String>,
    ) -> Self {
        Self {
            visual_kind: VisualDatasetKind::Scatter,
            source: DatasetSource::LocalParquet { path, limit },
            row_count,
            field_bindings: vec![
                DatasetFieldBinding::new(DatasetFieldRole::X, x_column),
                DatasetFieldBinding::new(DatasetFieldRole::Y, y_column),
            ],
            lane_labels: Vec::new(),
        }
    }

    /// Identity for a local Parquet timeline dataset.
    pub fn local_parquet_timeline(
        path: PathBuf,
        row_count: usize,
        limit: Option<usize>,
        time_column: impl Into<String>,
        lane_column: impl Into<String>,
        lane_labels: Vec<String>,
    ) -> Self {
        Self {
            visual_kind: VisualDatasetKind::Timeline,
            source: DatasetSource::LocalParquet { path, limit },
            row_count,
            field_bindings: vec![
                DatasetFieldBinding::new(DatasetFieldRole::Time, time_column),
                DatasetFieldBinding::new(DatasetFieldRole::Lane, lane_column),
            ],
            lane_labels,
        }
    }
}
