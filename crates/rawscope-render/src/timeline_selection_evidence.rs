//! Render-owned adapter from timeline brush state into evidence-owned models.

use rawscope_data::{SyntheticDatasetMetadata, TimelineEventRecord};
use rawscope_evidence::{
    insert_lowest_row_id_sample, SelectedTimelineEventSample, TimelineEvidenceConfig,
    TimelineSelectionEvidence,
};

use crate::{TimelineBrushSelection, TimelineSelectionSummary};

/// Builds evidence from the current render-owned selection and summary contracts.
pub fn timeline_selection_evidence_from_events(
    events: &[TimelineEventRecord],
    selection: TimelineBrushSelection,
    lane_count: u32,
    dataset_metadata: SyntheticDatasetMetadata,
    event_count: usize,
    config: TimelineEvidenceConfig,
) -> TimelineSelectionEvidence {
    let summary = TimelineSelectionSummary::from_events(events, selection, lane_count);
    let mut selected_event_sample = Vec::new();

    for event in events {
        if !selection.contains_event(event) {
            continue;
        }
        insert_lowest_row_id_sample(
            &mut selected_event_sample,
            SelectedTimelineEventSample::from(event),
            config.max_sample_size,
        );
    }

    let selected_row_id_sample = selected_event_sample
        .iter()
        .map(|sample| sample.row_id)
        .collect();

    TimelineSelectionEvidence::from_parts(
        dataset_metadata,
        event_count,
        summary.selected_event_count,
        summary.selected_percentage,
        summary.selected_time_range,
        summary.selected_lane_range,
        selected_row_id_sample,
        selected_event_sample,
        summary.lane_counts,
        summary.event_type_counts,
        summary.top_lane,
        summary.top_event_type,
        summary.selected_timestamp_range,
        summary.selected_value_range,
    )
}
