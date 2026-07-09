//! Synthetic and local datasets for RawScope.

mod dataset;
mod local_dataset;
mod synthetic;
mod visual_record;

pub use dataset::{
    DatasetFieldBinding, DatasetFieldRole, DatasetIdentity, DatasetSource,
    SyntheticDatasetMetadata, VisualDatasetKind,
};
pub use local_dataset::{
    load_parquet_scatter_dataset, load_parquet_timeline_dataset, load_scatter_dataset,
    load_timeline_dataset, DatasetChunkId, DatasetLoadError, LoadedColumnKind, LoadedColumnSchema,
    LoadedColumnarChunk, LoadedColumnarDataset, LoadedScatterDataset, LoadedSourceRow,
    LoadedSourceTable, LoadedTimelineDataset,
};
pub use synthetic::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticEventDataset, SyntheticEventType, SyntheticPointCategory, SyntheticPointConfig,
    SyntheticPointDataset,
};
pub use visual_record::{
    ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};
