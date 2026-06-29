//! Synthetic datasets and dataset metadata for RawScope.

mod dataset;
mod synthetic;

pub use dataset::SyntheticDatasetMetadata;
pub use synthetic::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticEventDataset, SyntheticEventRecord, SyntheticEventType, SyntheticPointCategory,
    SyntheticPointConfig, SyntheticPointDataset, SyntheticPointRecord,
};

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "synthetic datasets, dataset metadata, and future columnar data abstractions"
}
