//! Synthetic and local datasets for RawScope.

mod dataset;
mod dataset_profile;
mod local_dataset;
mod synthetic;
mod visual_record;

pub use dataset::{
    DatasetFieldBinding, DatasetFieldRole, DatasetIdentity, DatasetSource,
    SyntheticDatasetMetadata, VisualDatasetKind,
};
pub use dataset_profile::{
    dataset_profile, parse_dataset_profile_id, supported_dataset_profile_ids,
    validate_dataset_profile, DatasetProfile, DatasetProfileColumn, DatasetProfileId,
    DatasetProfileParseError, DatasetProfileValidationError, DatasetProfileValidationMismatch,
    ScatterDatasetProfileBinding, TimelineDatasetProfileBinding,
};
pub use local_dataset::{
    load_dataset_schema, load_parquet_scatter_dataset, load_parquet_timeline_dataset,
    load_scatter_dataset, load_timeline_dataset, DatasetChunkId, DatasetLoadError,
    LoadedColumnKind, LoadedColumnSchema, LoadedColumnarChunk, LoadedColumnarDataset,
    LoadedScatterDataset, LoadedSourceRow, LoadedSourceTable, LoadedTimelineDataset,
};
pub use synthetic::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticEventDataset, SyntheticEventType, SyntheticPointCategory, SyntheticPointConfig,
    SyntheticPointDataset,
};
pub use visual_record::{
    ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};
