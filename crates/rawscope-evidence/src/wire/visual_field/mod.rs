//! Versioned generic visual-field evidence wire contracts.

pub mod v1;

pub use v1::{
    visual_field_evidence_from_json, visual_field_evidence_json,
    visual_field_evidence_v1_from_json, visual_field_evidence_v1_json,
    CategoryCompositionEvidenceV1, CategoryLayerEvidenceV1, CategoryReservedLayerV1,
    CohortComparisonEvidenceV1, CohortEvidenceV1, ComparisonSplitEvidenceV1, DensityModeEvidenceV1,
    DensityRidgeEvidenceV1, EvidenceConversionError, EvidenceWireError, ExactFieldEvidenceV1,
    FieldDomainEvidenceV1, FieldEvidenceV1, FieldKindEvidenceV1, FreshnessEvidenceV1,
    MassContourEvidenceV1, NumericDomainEvidenceV1, PinnedCellDetailEvidenceV1,
    QualityTierEvidenceV1, RowCountAggregationEvidenceV1, SelectionEvidenceV1,
    SemanticPointSampleEvidenceV1, SourceEvidenceV1, TimeQuantizationEvidenceV1,
    VisualFieldEvidenceV1, VisualFieldModeEvidenceV1, VisualFieldPinnedCellEvidenceV1,
    VisualFieldProjectionEvidenceV1, VisualFieldProvenanceEvidenceV1,
    VISUAL_FIELD_EVIDENCE_V1_ARTIFACT_KIND, VISUAL_FIELD_EVIDENCE_V1_SCHEMA,
    VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION,
};
