//! Synthetic and local datasets for RawScope.

mod dataset;
mod local_dataset;
mod synthetic;

pub use dataset::{
    DatasetFieldBinding, DatasetFieldRole, DatasetIdentity, DatasetSource,
    SyntheticDatasetMetadata, VisualDatasetKind,
};
pub use local_dataset::{
    load_scatter_dataset, load_timeline_dataset, DatasetLoadError, LoadedColumnKind,
    LoadedColumnSchema, LoadedScatterDataset, LoadedTimelineDataset,
};
pub use synthetic::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticEventDataset, SyntheticEventRecord, SyntheticEventType, SyntheticPointCategory,
    SyntheticPointConfig, SyntheticPointDataset, SyntheticPointRecord,
};
