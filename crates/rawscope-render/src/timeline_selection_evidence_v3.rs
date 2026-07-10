//! Timeline selection evidence v3 models with visual encoding and aggregate context.

use rawscope_core::{RowId, U64Range};
use rawscope_data::{DatasetIdentity, DatasetProfileId};

use crate::scatter_selection_evidence_v3::AggregateEvidenceBin;
use crate::{
    DensityEncoding, SelectedSourceRowSample, SelectedTimelineEventSampleV2, TimelineLaneRange,
    TimelineSelectionComparison, TimelineSelectionEvidenceV2,
};

/// Schema version for timeline selection evidence v3 artifacts.
pub const TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION: u32 = 3;

/// View configuration included in timeline evidence v3 artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEvidenceViewV3 {
    pub time_range: U64Range,
    pub full_time_range: U64Range,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub density_encoding: DensityEncoding,
}

/// Bounded aggregate context retained in timeline evidence v3 artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineAggregateEvidenceContext {
    pub bin_limit: usize,
    pub bins: Vec<AggregateEvidenceBin>,
}

/// Source-aware CPU-side timeline selection evidence for v3 artifacts.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSelectionEvidenceV3 {
    pub schema_version: u32,
    pub dataset_identity: DatasetIdentity,
    pub active_dataset_profile: Option<DatasetProfileId>,
    pub view: TimelineEvidenceViewV3,
    pub selected_event_count: usize,
    pub selected_percentage: f32,
    pub selected_time_range: U64Range,
    pub selected_lane_range: TimelineLaneRange,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_event_sample: Vec<SelectedTimelineEventSampleV2>,
    pub selected_source_column_names: Vec<String>,
    pub selected_source_row_sample: Vec<SelectedSourceRowSample>,
    pub comparison: TimelineSelectionComparison,
    pub aggregate_context: TimelineAggregateEvidenceContext,
}

impl TimelineSelectionEvidenceV3 {
    /// Extends v2 evidence with encoding, comparison, and bounded aggregate context.
    pub fn from_v2(
        evidence: &TimelineSelectionEvidenceV2,
        density_encoding: DensityEncoding,
        comparison: TimelineSelectionComparison,
        aggregate_context: TimelineAggregateEvidenceContext,
        active_dataset_profile: Option<DatasetProfileId>,
    ) -> Self {
        Self {
            schema_version: TIMELINE_SELECTION_EVIDENCE_V3_SCHEMA_VERSION,
            dataset_identity: evidence.dataset_identity.clone(),
            active_dataset_profile,
            view: TimelineEvidenceViewV3 {
                time_range: evidence.view.time_range,
                full_time_range: evidence.view.full_time_range,
                lane_count: evidence.view.lane_count,
                grid_width: evidence.view.grid_width,
                grid_height: evidence.view.grid_height,
                density_encoding,
            },
            selected_event_count: evidence.selected_event_count,
            selected_percentage: evidence.selected_percentage,
            selected_time_range: evidence.selected_time_range,
            selected_lane_range: evidence.selected_lane_range,
            selected_row_id_sample: evidence.selected_row_id_sample.clone(),
            selected_event_sample: evidence.selected_event_sample.clone(),
            selected_source_column_names: evidence.selected_source_column_names.clone(),
            selected_source_row_sample: evidence.selected_source_row_sample.clone(),
            comparison,
            aggregate_context,
        }
    }
}
