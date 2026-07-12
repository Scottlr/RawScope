//! Scatter selection evidence v3 models with visual encoding and aggregate context.

use rawscope_core::{F32Range, RowId};
use rawscope_data::{DatasetIdentity, DatasetProfileId};

use crate::{
    DensityEncoding, ScatterAggregateEvidenceContext, ScatterDensityPresentation,
    ScatterSelectionComparison, ScatterSelectionEvidenceV2, SelectedPointSampleV2,
    SelectedSourceRowSample,
};

/// Schema version for scatter selection evidence v3 artifacts.
pub const SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION: u32 = 3;

/// View configuration included in scatter evidence v3 artifacts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterEvidenceViewV3 {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub density_encoding: DensityEncoding,
    pub density_presentation: ScatterDensityPresentation,
}

/// Source-aware CPU-side scatter selection evidence for v3 artifacts.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionEvidenceV3 {
    pub schema_version: u32,
    pub dataset_identity: DatasetIdentity,
    pub active_dataset_profile: Option<DatasetProfileId>,
    pub view: ScatterEvidenceViewV3,
    pub selected_row_count: usize,
    pub selected_percentage: f32,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_record_sample: Vec<SelectedPointSampleV2>,
    pub selected_source_column_names: Vec<String>,
    pub selected_source_row_sample: Vec<SelectedSourceRowSample>,
    pub comparison: ScatterSelectionComparison,
    pub aggregate_context: ScatterAggregateEvidenceContext,
}

impl ScatterSelectionEvidenceV3 {
    /// Extends v2 evidence with encoding, comparison, and bounded aggregate context.
    pub fn from_v2(
        evidence: &ScatterSelectionEvidenceV2,
        density_encoding: DensityEncoding,
        comparison: ScatterSelectionComparison,
        aggregate_context: ScatterAggregateEvidenceContext,
        active_dataset_profile: Option<DatasetProfileId>,
    ) -> Self {
        Self::from_v2_with_presentation(
            evidence,
            density_encoding,
            ScatterDensityPresentation::ExactCells,
            comparison,
            aggregate_context,
            active_dataset_profile,
        )
    }

    /// Extends v2 evidence with an explicit scatter presentation mode.
    pub fn from_v2_with_presentation(
        evidence: &ScatterSelectionEvidenceV2,
        density_encoding: DensityEncoding,
        density_presentation: ScatterDensityPresentation,
        comparison: ScatterSelectionComparison,
        aggregate_context: ScatterAggregateEvidenceContext,
        active_dataset_profile: Option<DatasetProfileId>,
    ) -> Self {
        Self {
            schema_version: SCATTER_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
            dataset_identity: evidence.dataset_identity.clone(),
            active_dataset_profile,
            view: ScatterEvidenceViewV3 {
                x_range: evidence.view.x_range,
                y_range: evidence.view.y_range,
                grid_width: evidence.view.grid_width,
                grid_height: evidence.view.grid_height,
                density_encoding,
                density_presentation,
            },
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            selected_row_id_sample: evidence.selected_row_id_sample.clone(),
            selected_record_sample: evidence.selected_record_sample.clone(),
            selected_source_column_names: evidence.selected_source_column_names.clone(),
            selected_source_row_sample: evidence.selected_source_row_sample.clone(),
            comparison,
            aggregate_context,
        }
    }
}
