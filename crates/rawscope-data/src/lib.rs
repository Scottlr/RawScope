//! Synthetic and local datasets for RawScope.

mod dataset;
mod local_dataset;
mod synthetic;

pub use dataset::SyntheticDatasetMetadata;
pub use local_dataset::{
    load_scatter_dataset, load_timeline_dataset, DatasetLoadError, LoadedColumnKind,
    LoadedColumnSchema, LoadedScatterDataset, LoadedTimelineDataset,
};
pub use synthetic::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticEventDataset, SyntheticEventRecord, SyntheticEventType, SyntheticPointCategory,
    SyntheticPointConfig, SyntheticPointDataset, SyntheticPointRecord,
};

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "synthetic datasets, local CSV loading, dataset metadata, and future columnar data abstractions"
}
