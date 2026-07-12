//! Synthetic and local datasets for RawScope.

mod dataset;
mod dataset_filter;
mod dataset_profile;
mod evidence_key;
mod generation_admission;
mod ingest;
mod local_dataset;
mod scatter_projection;
mod store;
mod synthetic;
mod visual_field_catalog;
mod visual_record;

pub use dataset::{
    DatasetFieldBinding, DatasetFieldRole, DatasetIdentity, DatasetSource, DatasetSourceFormat,
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
    ScatterDatasetProfileDefaults, ScatterDatasetProfileHints, TimelineDatasetProfileBinding,
};
pub use evidence_key::{
    validate_evidence_key, validate_evidence_key_with_budget, BoundDatasetEvidenceKey,
    BoundEvidenceKeyError, DatasetEvidenceKey, EvidenceKeyIndexBudget, EvidenceKeyValidationError,
};
pub use generation_admission::{
    AdmissionError, DatasetGenerationAdmission, PendingResourceReservation, ResourceBudget,
    ResourceReservation,
};
pub use ingest::{
    CancellationCheck, ChunkRowCount, IngestionError, IngestionPlan, IngestionProgress,
    IngestionRequest,
};
pub use local_dataset::{
    load_dataset_schema, load_scatter_dataset, load_timeline_dataset, DatasetChunkId,
    DatasetLoadError, LoadedColumnKind, LoadedColumnSchema, LoadedColumnarChunk,
    LoadedColumnarDataset, LoadedScatterDataset, LoadedSourceRow, LoadedSourceTable,
    LoadedTimelineDataset,
};
#[cfg(feature = "parquet")]
pub use local_dataset::{load_parquet_scatter_dataset, load_parquet_timeline_dataset};
pub use scatter_projection::{
    project_scatter_points, ProjectedScatterData, ScatterProjection, ScatterProjectionError,
    ScatterProjectionLabels, ScatterProjectionSpec,
};
pub use store::{
    CellRef, CellState, ChunkValidationError, ColumnChunk, DatasetAccessError, DatasetBudgetError,
    DatasetChunk, DatasetColumn, DatasetGeneration, DatasetGenerationCounter, DatasetMemoryBudget,
    DatasetMemoryUsage, DatasetSchema, DatasetSchemaError, DatasetStore, DatasetStoreBuilder,
    DatasetStoreError, DatasetStoreOwner, DecodedCsvCell, InvalidCell, InvalidCellReason,
    NormalizedValue, SourceUnavailableReason, SourceValue, StoreColumnKind, StoredCell,
};
pub use synthetic::{
    generate_synthetic_events, generate_synthetic_points, try_generate_synthetic_events,
    SyntheticEventConfig, SyntheticEventConfigError, SyntheticEventDataset, SyntheticEventType,
    SyntheticPointCategory, SyntheticPointConfig, SyntheticPointDataset,
    ValidatedSyntheticEventConfig,
};
pub use visual_field_catalog::{
    build_visual_field_catalog, CategoricalFieldSummary, CategoryValueCount, NumericFieldSummary,
    VisualFieldCatalog, VisualFieldCatalogConfig, VisualFieldDescriptor, VisualFieldSummary,
};
pub use visual_record::{
    ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};
