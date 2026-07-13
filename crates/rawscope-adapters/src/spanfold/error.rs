//! Errors for SpanFold transformation and session materialization.

use std::{io, path::PathBuf};

use rawscope_session::SessionManifestError;
use spanfold::ComparisonRowMetadataError;
use thiserror::Error;

/// Failure to transform a SpanFold result or prepare its RawScope session bundle.
#[derive(Debug, Error)]
pub enum SpanfoldAdapterError {
    /// SpanFold rejected the comparison before rows could be adapted.
    #[error("SpanFold comparison is invalid; diagnostics: {diagnostics:?}")]
    InvalidComparison { diagnostics: Vec<String> },

    /// The configured row families produced no interval records.
    #[error("SpanFold comparison contains no selected interval rows")]
    NoSelectedIntervals,

    /// SpanFold rows and their authoritative metadata no longer share a valid layout.
    #[error(transparent)]
    InconsistentRowMetadata(#[from] ComparisonRowMetadataError),

    /// A future SpanFold temporal axis is not yet mapped by this adapter.
    #[error("SpanFold temporal axis '{axis}' is not supported by this adapter")]
    UnsupportedTemporalAxis { axis: String },

    /// Selected rows use different temporal axes or clock identities.
    #[error(
        "SpanFold intervals mix temporal domains: expected {expected_axis}/{expected_clock:?}, got {actual_axis}/{actual_clock:?}"
    )]
    MixedTemporalDomain {
        expected_axis: &'static str,
        expected_clock: Option<String>,
        actual_axis: &'static str,
        actual_clock: Option<String>,
    },

    /// A row range cannot be represented as a non-negative exact duration.
    #[error("SpanFold {family} row has an invalid or overflowing range {start}..{end}")]
    InvalidRange {
        family: &'static str,
        start: i64,
        end: i64,
    },

    /// Source record identifiers could not be encoded into one retained CSV cell.
    #[error("failed to encode record identifiers for SpanFold {family} row")]
    RecordIdentifiers {
        family: &'static str,
        #[source]
        source: serde_json::Error,
    },

    /// The caller-selected destination already exists and will not be overwritten.
    #[error("RawScope SpanFold session destination already exists: '{}'", path.display())]
    DestinationExists { path: PathBuf },

    /// The destination cannot be represented as a sibling staging directory.
    #[error(
        "RawScope SpanFold session destination has no usable final path component: '{}'",
        path.display()
    )]
    InvalidDestination { path: PathBuf },

    /// Filesystem IO failed with path and operation context.
    #[error("failed to {operation} '{}': {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// CSV serialization failed for the materialized interval table.
    #[error("failed to write SpanFold CSV dataset '{}': {source}", path.display())]
    Csv {
        path: PathBuf,
        #[source]
        source: csv::Error,
    },

    /// Cleanup failed after another preparation error.
    #[error(
        "SpanFold session preparation failed ({original}); staging cleanup also failed for '{}': {source}",
        path.display()
    )]
    Cleanup {
        path: PathBuf,
        original: String,
        #[source]
        source: io::Error,
    },

    /// The generated session manifest failed the shared RawScope contract.
    #[error(transparent)]
    Session(#[from] SessionManifestError),
}
