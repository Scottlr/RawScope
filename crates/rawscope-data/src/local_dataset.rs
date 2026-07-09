//! Local dataset loading types for the first real-data workbench slice.

use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use rawscope_core::{F32Range, U64Range};

use crate::{DatasetIdentity, ScatterPointRecord, TimelineEventRecord};

mod csv;

pub use csv::{load_scatter_dataset, load_timeline_dataset};

pub(super) const CSV_EXTENSION: &str = "csv";
pub(super) const PARQUET_EXTENSION: &str = "parquet";
pub(super) const SCATTER_NUMERIC_TYPE_EXPECTATION: &str =
    "integer or float-compatible numeric values";
pub(super) const TIMELINE_TIME_TYPE_EXPECTATION: &str = "integer timestamp values";
pub(super) const TIMELINE_LANE_TYPE_EXPECTATION: &str =
    "string or non-negative integer lane values";

/// Inferred type summary for a loaded local column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadedColumnKind {
    Empty,
    Integer,
    Float,
    String,
}

impl LoadedColumnKind {
    pub(super) fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Float)
    }

    pub(super) fn is_integer(self) -> bool {
        self == Self::Integer
    }

    pub(super) fn is_string_or_integer(self) -> bool {
        matches!(self, Self::Integer | Self::String)
    }
}

impl fmt::Display for LoadedColumnKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty"),
            Self::Integer => write!(f, "integer"),
            Self::Float => write!(f, "float"),
            Self::String => write!(f, "string"),
        }
    }
}

/// Name and inferred type for a loaded local dataset column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedColumnSchema {
    pub name: String,
    pub kind: LoadedColumnKind,
}

/// Loaded scatter-ready dataset mapped into shared visual point records.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedScatterDataset {
    pub identity: DatasetIdentity,
    pub schema: Vec<LoadedColumnSchema>,
    pub x_column: String,
    pub y_column: String,
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub points: Vec<ScatterPointRecord>,
}

/// Loaded timeline-ready dataset mapped into shared visual event records.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedTimelineDataset {
    pub identity: DatasetIdentity,
    pub schema: Vec<LoadedColumnSchema>,
    pub time_column: String,
    pub lane_column: String,
    pub time_range: U64Range,
    pub lane_count: u32,
    pub lane_labels: Vec<String>,
    pub events: Vec<TimelineEventRecord>,
}

/// Errors returned while opening a local dataset and binding columns.
#[derive(Debug)]
pub enum DatasetLoadError {
    CsvRead {
        path: PathBuf,
        source: ::csv::Error,
    },
    ParquetDeferred {
        path: PathBuf,
    },
    UnsupportedFileFormat {
        path: PathBuf,
        extension: Option<String>,
    },
    MissingColumn {
        column: String,
        available_columns: Vec<String>,
    },
    UnsupportedColumnType {
        column: String,
        expected: &'static str,
        actual: LoadedColumnKind,
    },
    InvalidColumnValue {
        column: String,
        row_number: usize,
        value: String,
        expected: &'static str,
    },
    EmptyDataset {
        path: PathBuf,
    },
    TooManyLanes {
        lane_count: usize,
    },
}

impl fmt::Display for DatasetLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CsvRead { path, source } => {
                write!(f, "failed to read CSV '{}': {source}", path.display())
            }
            Self::ParquetDeferred { path } => write!(
                f,
                "Parquet import is not included in Milestone 5A yet for '{}'; use CSV for this slice",
                path.display()
            ),
            Self::UnsupportedFileFormat { path, extension } => {
                let extension = extension.as_deref().unwrap_or("<none>");
                write!(
                    f,
                    "unsupported dataset file extension '{extension}' for '{}'; CSV is supported and Parquet is deferred",
                    path.display()
                )
            }
            Self::MissingColumn {
                column,
                available_columns,
            } => write!(
                f,
                "missing column '{column}'; available columns: {}",
                available_columns.join(", ")
            ),
            Self::UnsupportedColumnType {
                column,
                expected,
                actual,
            } => write!(
                f,
                "unsupported type for column '{column}': expected {expected}, inferred {actual}"
            ),
            Self::InvalidColumnValue {
                column,
                row_number,
                value,
                expected,
            } => write!(
                f,
                "invalid value in column '{column}' at CSV row {row_number}: expected {expected}, got '{value}'"
            ),
            Self::EmptyDataset { path } => {
                write!(f, "dataset '{}' contains no rows", path.display())
            }
            Self::TooManyLanes { lane_count } => write!(
                f,
                "timeline dataset has {lane_count} distinct lanes, which exceeds u32 lane support"
            ),
        }
    }
}

impl Error for DatasetLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CsvRead { source, .. } => Some(source),
            Self::ParquetDeferred { .. }
            | Self::UnsupportedFileFormat { .. }
            | Self::MissingColumn { .. }
            | Self::UnsupportedColumnType { .. }
            | Self::InvalidColumnValue { .. }
            | Self::EmptyDataset { .. }
            | Self::TooManyLanes { .. } => None,
        }
    }
}

pub(super) fn ensure_supported_csv(path: &Path) -> Result<(), DatasetLoadError> {
    let extension = normalized_extension(path);
    match extension.as_deref() {
        Some(CSV_EXTENSION) => Ok(()),
        Some(PARQUET_EXTENSION) => Err(DatasetLoadError::ParquetDeferred {
            path: path.to_path_buf(),
        }),
        _ => Err(DatasetLoadError::UnsupportedFileFormat {
            path: path.to_path_buf(),
            extension,
        }),
    }
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}
