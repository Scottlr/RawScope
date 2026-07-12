//! Private validated evidence semantics, independent of wire formats.

mod aggregate_evidence;
mod bundle;
mod comparison;
mod density_encoding;
mod document;
mod evidence_sample;
mod markdown_escape;
mod provenance;
mod quantization;
mod scatter_density_presentation;
mod scatter_selection_evidence;
mod scatter_selection_evidence_v3;
mod scatter_selection_evidence_v4;
mod scatter_selection_evidence_v5;
mod scatter_selection_export;
mod scatter_selection_export_v3;
mod scatter_selection_export_v4;
mod scatter_selection_export_v5;
mod scatter_visual_support;
mod schema;
mod timeline_selection_evidence;
mod timeline_selection_evidence_v3;
mod timeline_selection_export;
mod timeline_selection_export_v3;
pub mod wire;

pub use evidence_sample::{insert_lowest_row_id_sample, RowIdSample};
pub use markdown_escape::{escape_inline_code, escape_table_cell, portable_path_label};

pub use aggregate_evidence::{
    AggregateEvidenceBin, ScatterAggregateEvidenceContext, TimelineAggregateEvidenceContext,
};
pub use bundle::{
    BundleArtifactV2, BundleManifestError, BundleManifestV2, BundlePathError, BundleRelativePath,
    ContentChecksumV1, EvidenceArtifactKind, REPORT_BUNDLE_SCHEMA_VERSION_V2,
};
pub use comparison::{
    ComparisonRatio, ScatterKindComparison, ScatterSelectionComparison, TimelineKindComparison,
    TimelineSelectionComparison,
};
pub use density_encoding::{
    density_intensity, DensityEncoding, DensityNormalization, DensityPalette, DensityTransform,
};
pub use document::{
    EvidenceContext, EvidenceDocument, EvidenceValidationError, EvidenceVisualContext,
};
pub use provenance::{
    ContentFingerprintV1, EvidenceProvenanceV1, FingerprintAlgorithm, FingerprintCoverage,
    ProvenancePolicy,
};
pub use quantization::{
    GpuNumericEncoding, NumericDomainV1, QuantizationDisclosureV1, QuantizationError,
    SourceNumericType,
};
pub use scatter_density_presentation::ScatterDensityPresentation;
pub use scatter_selection_evidence::{
    ScatterEvidenceView, ScatterSelectionEvidence, ScatterSelectionEvidenceV2,
    ScatterSelectionGeometry, ScatterSelectionKindCounts, SelectedPointSample,
    SelectedPointSampleV2, SelectedSourceRowSample, SelectionEvidenceConfig,
};
pub use scatter_selection_evidence_v3::{
    ScatterEvidenceViewV3, ScatterSelectionEvidenceV3, SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use scatter_selection_evidence_v4::{
    DifferenceDensityEvidenceConfig, PinnedScatterInspectionEvidence, PointRevealEvidence,
    ScatterCohortEvidence, ScatterSelectionEvidenceV4, ScatterSelectionEvidenceV4Error,
    ScatterVisualQueryV4, DIFFERENCE_BASELINE_ID, DIFFERENCE_FORMULA_ID,
    SCATTER_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
};
pub use scatter_selection_evidence_v5::{
    DifferenceDirectionEvidenceV5, PinnedDifferenceInspectionEvidenceV5,
    PinnedScatterInspectionEvidenceV5, ScatterSelectionEvidenceV5, ScatterSelectionEvidenceV5Error,
    SessionDataFormatEvidenceV5, SessionEvidenceContextV5,
    SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION,
};
pub use scatter_selection_export::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown,
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
    SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
pub use scatter_selection_export_v3::{
    scatter_selection_evidence_v3_json, scatter_selection_evidence_v3_markdown,
    ScatterSelectionExportError, SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
pub use scatter_selection_export_v4::{
    scatter_selection_evidence_v4_json, scatter_selection_evidence_v4_markdown,
    SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND,
};
pub use scatter_selection_export_v5::{
    scatter_selection_evidence_v5_json, scatter_selection_evidence_v5_markdown,
    SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND,
};
pub use scatter_visual_support::{
    validate_relief_field_config, PointRevealMode, ReliefFieldConfig, ReliefFieldConfigError,
    ScatterDensityMode, INSPECTION_NEIGHBORHOOD_RADIUS_BINS, MAX_RELIEF_ELEVATION_DEGREES,
    MAX_RELIEF_HEIGHT_STRENGTH, MAX_RELIEF_NORMAL_RADIUS_BINS, MIN_RELIEF_ELEVATION_DEGREES,
    MIN_RELIEF_HEIGHT_STRENGTH, MIN_RELIEF_NORMAL_RADIUS_BINS,
};
pub use schema::{
    EvidenceSchemaFamily, EvidenceSchemaVersion, SCATTER_SELECTION_EVIDENCE_V6_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
};
pub use timeline_selection_evidence::{
    SelectedEventTypeCounts, SelectedTimelineEventSample, SelectedTimelineEventSampleV2,
    TimelineEvidenceConfig, TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidence,
    TimelineSelectionEvidenceV2,
};
pub use timeline_selection_evidence_v3::{
    TimelineEvidenceViewV3, TimelineSelectionEvidenceV3,
    TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
};
pub use timeline_selection_export::{
    timeline_selection_evidence_json, timeline_selection_evidence_markdown,
    timeline_selection_evidence_v2_json, timeline_selection_evidence_v2_markdown,
    TIMELINE_SELECTION_EVIDENCE_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
pub use timeline_selection_export_v3::{
    timeline_selection_evidence_v3_json, timeline_selection_evidence_v3_markdown,
    TimelineSelectionExportError, TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
};
