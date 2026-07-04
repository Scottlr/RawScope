//! CPU-side event evidence for finalized timeline brush selections.

use rawscope_core::{RowId, U64Range};
use rawscope_data::{SyntheticDatasetMetadata, SyntheticEventRecord, SyntheticEventType};

use crate::evidence_sample::{insert_lowest_row_id_sample, RowIdSample};
use crate::{
    SelectedEventTypeCounts, TimelineBrushSelection, TimelineLaneRange, TimelineSelectionSummary,
};

const DEFAULT_MAX_TIMELINE_SAMPLE_SIZE: usize = 10;

/// Configuration for deterministic timeline selection evidence building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEvidenceConfig {
    pub max_sample_size: usize,
}

impl Default for TimelineEvidenceConfig {
    fn default() -> Self {
        Self {
            max_sample_size: DEFAULT_MAX_TIMELINE_SAMPLE_SIZE,
        }
    }
}

/// Small sampled synthetic timeline event included in selection evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedTimelineEventSample {
    pub row_id: RowId,
    pub timestamp: u64,
    pub lane: u32,
    pub value: f32,
    pub event_type: SyntheticEventType,
}

impl From<&SyntheticEventRecord> for SelectedTimelineEventSample {
    fn from(event: &SyntheticEventRecord) -> Self {
        Self {
            row_id: event.row_id,
            timestamp: event.timestamp,
            lane: event.lane,
            value: event.value,
            event_type: event.event_type,
        }
    }
}

impl RowIdSample for SelectedTimelineEventSample {
    fn row_id(&self) -> RowId {
        self.row_id
    }
}

/// Deterministic CPU-side evidence for a finalized timeline brush selection.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSelectionEvidence {
    pub dataset_metadata: SyntheticDatasetMetadata,
    pub event_count: usize,
    pub selected_event_count: usize,
    pub selected_percentage: f32,
    pub selected_time_range: U64Range,
    pub selected_lane_range: TimelineLaneRange,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_event_sample: Vec<SelectedTimelineEventSample>,
    pub lane_counts: Vec<usize>,
    pub event_type_counts: SelectedEventTypeCounts,
    pub top_lane: Option<u32>,
    pub top_event_type: Option<SyntheticEventType>,
    pub selected_timestamp_range: Option<U64Range>,
    pub selected_value_range: Option<(f32, f32)>,
}

impl TimelineSelectionEvidence {
    /// Builds deterministic selected-event evidence from synthetic timeline event records.
    ///
    /// Sampling uses the lowest selected row ids so the same selection and dataset always produce
    /// the same evidence without random state.
    pub fn from_events(
        events: &[SyntheticEventRecord],
        selection: TimelineBrushSelection,
        lane_count: u32,
        dataset_metadata: SyntheticDatasetMetadata,
        event_count: usize,
        config: TimelineEvidenceConfig,
    ) -> Self {
        let summary = TimelineSelectionSummary::from_events(events, selection, lane_count);
        let mut selected_event_sample = Vec::new();

        for event in events {
            let event_is_selected = selection.contains_event(event);
            if !event_is_selected {
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

        Self {
            dataset_metadata,
            event_count,
            selected_event_count: summary.selected_event_count,
            selected_percentage: summary.selected_percentage,
            selected_time_range: summary.selected_time_range,
            selected_lane_range: summary.selected_lane_range,
            selected_row_id_sample,
            selected_event_sample,
            lane_counts: summary.lane_counts,
            event_type_counts: summary.event_type_counts,
            top_lane: summary.top_lane,
            top_event_type: summary.top_event_type,
            selected_timestamp_range: summary.selected_timestamp_range,
            selected_value_range: summary.selected_value_range,
        }
    }
}
