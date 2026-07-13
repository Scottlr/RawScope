//! Comparison visual-field owner.

mod compute;
mod generation;
mod presentation;
mod resources;

pub use compute::ComparisonFieldRenderer;
pub use generation::{
    ComparisonCompatibility, ComparisonCompatibilityError, ComparisonFieldGeneration,
    ComparisonFieldSemantics,
};
pub use presentation::{
    comparison_inspection, derive_comparison_marginals, sample_split_fields,
    shared_density_maximum, ComparisonFieldRenderStats, ComparisonMarginalError,
    ComparisonMarginals, ComparisonPresentation, ComparisonPresentationError, ComparisonSplit,
    SupportAwareDifference,
};
