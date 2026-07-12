//! Private validated evidence semantics, independent of wire formats.

mod bundle;
mod document;
mod evidence_sample;
mod markdown_escape;
mod provenance;
mod quantization;
mod scatter_selection_evidence;
mod scatter_selection_export;
mod schema;
pub mod wire;

pub use bundle::{
    BundleArtifactV2, BundleManifestError, BundleManifestV2, BundlePathError, BundleRelativePath,
    ContentChecksumV1, EvidenceArtifactKind, REPORT_BUNDLE_SCHEMA_VERSION_V2,
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
pub use scatter_selection_evidence::{
    ScatterEvidenceView, ScatterSelectionEvidence, ScatterSelectionEvidenceV2,
    ScatterSelectionGeometry, ScatterSelectionKindCounts, SelectedPointSample,
    SelectedPointSampleV2, SelectedSourceRowSample, SelectionEvidenceConfig,
};
pub use scatter_selection_export::{
    scatter_selection_evidence_json, scatter_selection_evidence_markdown,
    scatter_selection_evidence_v2_json, scatter_selection_evidence_v2_markdown,
    SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
    SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND, SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
};
pub use schema::{
    EvidenceSchemaFamily, EvidenceSchemaVersion, SCATTER_SELECTION_EVIDENCE_V6_SCHEMA_VERSION,
    TIMELINE_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
};
