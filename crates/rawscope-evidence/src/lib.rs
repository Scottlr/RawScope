//! Private validated evidence semantics, independent of wire formats.

mod document;
mod provenance;
mod quantization;

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
