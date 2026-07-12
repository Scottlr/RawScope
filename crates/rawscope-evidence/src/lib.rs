//! Private validated evidence semantics, independent of wire formats.

mod bundle;
mod document;
mod provenance;
mod quantization;

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
