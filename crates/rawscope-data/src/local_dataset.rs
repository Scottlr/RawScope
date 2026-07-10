//! Local dataset loading types for the first real-data workbench slice.

use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use rawscope_core::{F32Range, RowId, U64Range};

use crate::{DatasetIdentity, ScatterPointRecord, TimelineEventRecord};

mod csv;
mod parquet;

pub use parquet::{load_parquet_scatter_dataset, load_parquet_timeline_dataset};

pub(super) const CSV_EXTENSION: &str = "csv";
pub(super) const PARQUET_EXTENSION: &str = "parquet";
pub(super) const SCATTER_NUMERIC_TYPE_EXPECTATION: &str =
    "integer or float-compatible numeric values";
pub(super) const TIMELINE_TIME_TYPE_EXPECTATION: &str = "integer timestamp values";
pub(super) const TIMELINE_LANE_TYPE_EXPECTATION: &str =
    "string or non-negative integer lane values";
pub(super) const PARQUET_RECORD_BATCH_SIZE_ROWS: usize = 1_024;

/// Stable chunk identifier for chunked local datasets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetChunkId(pub u32);

/// One chunk of loaded local columnar data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedColumnarChunk {
    pub chunk_id: DatasetChunkId,
    pub row_id_start: RowId,
    pub row_count: usize,
}

/// Chunked local dataset structure retained alongside row-oriented view records.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedColumnarDataset {
    pub identity: DatasetIdentity,
    pub schema: Vec<LoadedColumnSchema>,
    pub chunks: Vec<LoadedColumnarChunk>,
}

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

/// One retained local source row keyed by the loaded visual row id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSourceRow {
    pub row_id: RowId,
    pub values: Vec<String>,
}

/// Retained local source rows for future evidence and drilldown lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSourceTable {
    pub columns: Vec<LoadedColumnSchema>,
    pub rows: Vec<LoadedSourceRow>,
}

impl LoadedSourceTable {
    /// Looks up one retained source row by its visual row id.
    pub fn row(&self, row_id: RowId) -> Option<&LoadedSourceRow> {
        let index = usize::try_from(row_id.0).ok()?;
        self.rows.get(index).filter(|row| row.row_id == row_id)
    }

    /// Returns retained source column names in CSV order.
    pub fn column_names(&self) -> impl Iterator<Item = &str> {
        self.columns.iter().map(|column| column.name.as_str())
    }
}

/// Loaded scatter-ready dataset mapped into shared visual point records.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedScatterDataset {
    pub identity: DatasetIdentity,
    pub schema: Vec<LoadedColumnSchema>,
    pub columnar: Option<LoadedColumnarDataset>,
    pub source_rows: LoadedSourceTable,
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
    pub columnar: Option<LoadedColumnarDataset>,
    pub source_rows: LoadedSourceTable,
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
    ParquetRead {
        path: PathBuf,
        source: ::parquet::errors::ParquetError,
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
    UnsupportedParquetColumnType {
        column: String,
        expected: &'static str,
        actual: String,
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
            Self::ParquetRead { path, source } => {
                write!(f, "failed to read Parquet '{}': {source}", path.display())
            }
            Self::UnsupportedFileFormat { path, extension } => {
                let extension = extension.as_deref().unwrap_or("<none>");
                write!(
                    f,
                    "unsupported dataset file extension '{extension}' for '{}'; CSV and Parquet are supported",
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
            Self::UnsupportedParquetColumnType {
                column,
                expected,
                actual,
            } => write!(
                f,
                "unsupported Parquet type for column '{column}': expected {expected}, actual {actual}"
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
            Self::ParquetRead { source, .. } => Some(source),
            Self::UnsupportedParquetColumnType { .. }
            | Self::UnsupportedFileFormat { .. }
            | Self::MissingColumn { .. }
            | Self::UnsupportedColumnType { .. }
            | Self::InvalidColumnValue { .. }
            | Self::EmptyDataset { .. }
            | Self::TooManyLanes { .. } => None,
        }
    }
}

/// Loads a local scatter dataset from CSV or Parquet and binds explicit x/y columns.
pub fn load_scatter_dataset(
    path: impl AsRef<Path>,
    x_column: &str,
    y_column: &str,
    limit: Option<usize>,
) -> Result<LoadedScatterDataset, DatasetLoadError> {
    let path = path.as_ref();
    match normalized_extension(path).as_deref() {
        Some(CSV_EXTENSION) => csv::load_scatter_dataset(path, x_column, y_column, limit),
        Some(PARQUET_EXTENSION) => {
            parquet::load_parquet_scatter_dataset(path, x_column, y_column, limit)
        }
        _ => Err(DatasetLoadError::UnsupportedFileFormat {
            path: path.to_path_buf(),
            extension: normalized_extension(path),
        }),
    }
}

/// Loads only the inferred local dataset schema from CSV or Parquet.
pub fn load_dataset_schema(
    path: impl AsRef<Path>,
    limit: Option<usize>,
) -> Result<Vec<LoadedColumnSchema>, DatasetLoadError> {
    let path = path.as_ref();
    match normalized_extension(path).as_deref() {
        Some(CSV_EXTENSION) => csv::load_dataset_schema(path, limit),
        Some(PARQUET_EXTENSION) => parquet::load_dataset_schema(path, limit),
        _ => Err(DatasetLoadError::UnsupportedFileFormat {
            path: path.to_path_buf(),
            extension: normalized_extension(path),
        }),
    }
}

/// Loads a local timeline dataset from CSV or Parquet and binds explicit time/lane columns.
pub fn load_timeline_dataset(
    path: impl AsRef<Path>,
    time_column: &str,
    lane_column: &str,
    limit: Option<usize>,
) -> Result<LoadedTimelineDataset, DatasetLoadError> {
    let path = path.as_ref();
    match normalized_extension(path).as_deref() {
        Some(CSV_EXTENSION) => csv::load_timeline_dataset(path, time_column, lane_column, limit),
        Some(PARQUET_EXTENSION) => {
            parquet::load_parquet_timeline_dataset(path, time_column, lane_column, limit)
        }
        _ => Err(DatasetLoadError::UnsupportedFileFormat {
            path: path.to_path_buf(),
            extension: normalized_extension(path),
        }),
    }
}

pub(super) fn ensure_supported_csv(path: &Path) -> Result<(), DatasetLoadError> {
    let extension = normalized_extension(path);
    match extension.as_deref() {
        Some(CSV_EXTENSION) => Ok(()),
        Some(PARQUET_EXTENSION) => Err(DatasetLoadError::UnsupportedFileFormat {
            path: path.to_path_buf(),
            extension,
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
