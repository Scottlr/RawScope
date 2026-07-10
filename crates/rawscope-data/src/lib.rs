//! Synthetic and local datasets for RawScope.

mod dataset;
mod dataset_filter;
mod dataset_profile;
mod local_dataset;
mod synthetic;
mod visual_field_catalog;
mod visual_record;

pub use dataset::{
    DatasetFieldBinding, DatasetFieldRole, DatasetIdentity, DatasetSource,
    SyntheticDatasetMetadata, VisualDatasetKind,
};
pub use dataset_filter::{
    evaluate_filters, DatasetFilter, FilterError, FilterEvaluation, FilterMask, FilterRevision,
    FilterSet,
};
pub use dataset_profile::{
    available_profile_filter_hints, dataset_profile, parse_dataset_profile_id,
    supported_dataset_profile_ids, validate_dataset_profile, DatasetProfile, DatasetProfileColumn,
    DatasetProfileFilterHint, DatasetProfileFilterKind, DatasetProfileId, DatasetProfileParseError,
    DatasetProfileValidationError, DatasetProfileValidationMismatch, ScatterDatasetProfileBinding,
    ScatterDatasetProfileHints, TimelineDatasetProfileBinding,
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
pub use visual_field_catalog::{
    build_visual_field_catalog, CategoricalFieldSummary, CategoryValueCount, NumericFieldSummary,
    VisualFieldCatalog, VisualFieldCatalogConfig, VisualFieldDescriptor, VisualFieldSummary,
};
pub use visual_record::{
    ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};
