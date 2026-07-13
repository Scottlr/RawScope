//! Stable visual context shared by canonical selection and visual-field evidence.

use rawscope_analysis::cohort::CohortGeneration;
use rawscope_data::{DatasetGeneration, DatasetIdentity};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvidenceVisualContext {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub grid_width: u32,
    pub grid_height: u32,
}

#[derive(Debug, Clone)]
pub struct EvidenceContext {
    pub dataset_generation: DatasetGeneration,
    pub cohort_generation: CohortGeneration,
    pub dataset_identity: DatasetIdentity,
    pub cohort_included_row_count: u64,
    pub source_rows_available: bool,
    pub visual: EvidenceVisualContext,
}

pub(crate) fn valid_visual_context(context: EvidenceVisualContext) -> bool {
    context.grid_width > 0
        && context.grid_height > 0
        && context.x_min.is_finite()
        && context.x_max.is_finite()
        && context.y_min.is_finite()
        && context.y_max.is_finite()
        && context.x_max > context.x_min
        && context.y_max > context.y_min
}
