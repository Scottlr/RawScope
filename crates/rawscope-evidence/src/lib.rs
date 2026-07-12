//! Private validated evidence semantics, independent of wire formats.

mod document;

pub use document::{
    EvidenceContext, EvidenceDocument, EvidenceValidationError, EvidenceVisualContext,
};
