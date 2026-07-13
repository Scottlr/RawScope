//! Canonical evidence construction from immutable analytical truth.

mod selection;
mod visual_field;

pub use selection::{EvidenceDocument, EvidenceValidationError};
pub use visual_field::{EvidenceContext, EvidenceVisualContext};
