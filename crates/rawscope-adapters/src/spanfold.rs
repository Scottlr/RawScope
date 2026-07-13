//! SpanFold comparison transforms and RawScope session preparation.
//!
//! Callers run their SpanFold pipeline first, then adapt the comparison result
//! into an evidence-preserving rectangular table or a launchable RawScope bundle.

mod bundle;
mod error;
mod interval;
mod interval_mapping;

pub use bundle::{
    RAWSCOPE_SCATTER_DURATION_COLUMN, RAWSCOPE_SCATTER_START_COLUMN, SPANFOLD_DATASET_FILE_NAME,
    SPANFOLD_EVIDENCE_KEY_COLUMN, SPANFOLD_SESSION_MANIFEST_FILE_NAME,
};
pub use error::SpanfoldAdapterError;
pub use interval::{
    SpanfoldIntervalDataset, SpanfoldIntervalFamily, SpanfoldIntervalRow, SpanfoldIntervalTransform,
};
