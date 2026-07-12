//! CPU-side event evidence for finalized timeline brush selections.

use rawscope_core::{RowId, U64Range};
use rawscope_data::{
    DatasetIdentity, LoadedSourceTable, SyntheticDatasetMetadata, SyntheticEventType,
    TimelineEventKind, TimelineEventRecord,
};

use crate::RowIdSample;
use crate::SelectedSourceRowSample;

const DEFAULT_MAX_TIMELINE_SAMPLE_SIZE: usize = 10;

/// Half-open lane range selected by a finalized timeline brush.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineLaneRange {
    pub start: u32,
    pub end_exclusive: u32,
}

impl TimelineLaneRange {
    pub fn new(start: u32, end_exclusive: u32) -> Self {
        assert!(
            end_exclusive > start,
            "lane range end must be greater than start"
        );
        Self {
            start,
            end_exclusive,
        }
    }

    pub fn contains(self, lane: u32) -> bool {
        lane >= self.start && lane < self.end_exclusive
    }
}

/// Counts selected synthetic events by event kind.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SelectedEventTypeCounts {
    pub background: usize,
    pub spike: usize,
    pub stale_lane: usize,
    pub high_value_band: usize,
    pub unclassified: usize,
}

impl SelectedEventTypeCounts {
    pub fn top_event_type(self) -> Option<SyntheticEventType> {
        [
            (SyntheticEventType::Background, self.background),
            (SyntheticEventType::Spike, self.spike),
            (SyntheticEventType::StaleLane, self.stale_lane),
            (SyntheticEventType::HighValueBand, self.high_value_band),
        ]
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(event_type, count)| (count > 0).then_some(event_type))
    }

    pub fn top_event_kind(self) -> Option<TimelineEventKind> {
        [
            (
                TimelineEventKind::Synthetic(SyntheticEventType::Background),
                self.background,
            ),
            (
                TimelineEventKind::Synthetic(SyntheticEventType::Spike),
                self.spike,
            ),
            (
                TimelineEventKind::Synthetic(SyntheticEventType::StaleLane),
                self.stale_lane,
            ),
            (
                TimelineEventKind::Synthetic(SyntheticEventType::HighValueBand),
                self.high_value_band,
            ),
            (TimelineEventKind::Unclassified, self.unclassified),
        ]
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(kind, count)| (count > 0).then_some(kind))
    }

    pub fn add(&mut self, kind: TimelineEventKind) {
        match kind {
            TimelineEventKind::Synthetic(SyntheticEventType::Background) => self.background += 1,
            TimelineEventKind::Synthetic(SyntheticEventType::Spike) => self.spike += 1,
            TimelineEventKind::Synthetic(SyntheticEventType::StaleLane) => self.stale_lane += 1,
            TimelineEventKind::Synthetic(SyntheticEventType::HighValueBand) => {
                self.high_value_band += 1
            }
            TimelineEventKind::Unclassified => self.unclassified += 1,
        }
    }
}

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

/// Small sampled timeline event included in selection evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedTimelineEventSample {
    pub row_id: RowId,
    pub timestamp: u64,
    pub lane: u32,
    pub value: f32,
    pub event_type: Option<SyntheticEventType>,
}

impl From<&TimelineEventRecord> for SelectedTimelineEventSample {
    fn from(event: &TimelineEventRecord) -> Self {
        Self {
            row_id: event.row_id,
            timestamp: event.timestamp,
            lane: event.lane,
            value: event.value,
            event_type: event.kind.synthetic_event_type(),
        }
    }
}

/// View configuration included in timeline evidence v2 artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEvidenceView {
    pub time_range: U64Range,
    pub full_time_range: U64Range,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
}

/// Small sampled timeline event included in selection evidence v2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedTimelineEventSampleV2 {
    pub row_id: RowId,
    pub timestamp: u64,
    pub lane: u32,
    pub value: f32,
    pub kind: TimelineEventKind,
}

impl From<&SelectedTimelineEventSample> for SelectedTimelineEventSampleV2 {
    fn from(sample: &SelectedTimelineEventSample) -> Self {
        let kind = sample
            .event_type
            .map(TimelineEventKind::Synthetic)
            .unwrap_or(TimelineEventKind::Unclassified);
        Self {
            row_id: sample.row_id,
            timestamp: sample.timestamp,
            lane: sample.lane,
            value: sample.value,
            kind,
        }
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

/// Source-aware CPU-side timeline selection evidence for v2 artifacts.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSelectionEvidenceV2 {
    pub dataset_identity: DatasetIdentity,
    pub view: TimelineEvidenceView,
    pub selected_event_count: usize,
    pub selected_percentage: f32,
    pub selected_time_range: U64Range,
    pub selected_lane_range: TimelineLaneRange,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_event_sample: Vec<SelectedTimelineEventSampleV2>,
    pub selected_source_column_names: Vec<String>,
    pub selected_source_row_sample: Vec<SelectedSourceRowSample>,
    pub lane_counts: Vec<usize>,
    pub event_kind_counts: SelectedEventTypeCounts,
    pub top_lane: Option<u32>,
    pub top_event_kind: Option<TimelineEventKind>,
    pub selected_timestamp_range: Option<U64Range>,
    pub selected_value_range: Option<(f32, f32)>,
}

impl TimelineSelectionEvidenceV2 {
    /// Builds source-aware evidence from cached v1 evidence plus current dataset context.
    pub fn from_v1(
        evidence: &TimelineSelectionEvidence,
        dataset_identity: DatasetIdentity,
        view: TimelineEvidenceView,
        source_rows: Option<&LoadedSourceTable>,
    ) -> Self {
        let selected_source_column_names = source_rows
            .map(|table| table.column_names().map(str::to_string).collect())
            .unwrap_or_default();
        let selected_source_row_sample = source_rows
            .map(|table| {
                evidence
                    .selected_row_id_sample
                    .iter()
                    .filter_map(|row_id| table.row(*row_id).map(SelectedSourceRowSample::from))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            dataset_identity,
            view,
            selected_event_count: evidence.selected_event_count,
            selected_percentage: evidence.selected_percentage,
            selected_time_range: evidence.selected_time_range,
            selected_lane_range: evidence.selected_lane_range,
            selected_row_id_sample: evidence.selected_row_id_sample.clone(),
            selected_event_sample: evidence
                .selected_event_sample
                .iter()
                .map(SelectedTimelineEventSampleV2::from)
                .collect(),
            selected_source_column_names,
            selected_source_row_sample,
            lane_counts: evidence.lane_counts.clone(),
            event_kind_counts: evidence.event_type_counts,
            top_lane: evidence.top_lane,
            top_event_kind: evidence.event_type_counts.top_event_kind(),
            selected_timestamp_range: evidence.selected_timestamp_range,
            selected_value_range: evidence.selected_value_range,
        }
    }
}

impl RowIdSample for SelectedTimelineEventSample {
    fn row_id(&self) -> RowId {
        self.row_id
    }
}

impl TimelineSelectionEvidence {
    /// Creates evidence from the render-independent fields derived by the current owner.
    pub fn from_parts(
        dataset_metadata: SyntheticDatasetMetadata,
        event_count: usize,
        selected_event_count: usize,
        selected_percentage: f32,
        selected_time_range: U64Range,
        selected_lane_range: TimelineLaneRange,
        selected_row_id_sample: Vec<RowId>,
        selected_event_sample: Vec<SelectedTimelineEventSample>,
        lane_counts: Vec<usize>,
        event_type_counts: SelectedEventTypeCounts,
        top_lane: Option<u32>,
        top_event_type: Option<SyntheticEventType>,
        selected_timestamp_range: Option<U64Range>,
        selected_value_range: Option<(f32, f32)>,
    ) -> Self {
        Self {
            dataset_metadata,
            event_count,
            selected_event_count,
            selected_percentage,
            selected_time_range,
            selected_lane_range,
            selected_row_id_sample,
            selected_event_sample,
            lane_counts,
            event_type_counts,
            top_lane,
            top_event_type,
            selected_timestamp_range,
            selected_value_range,
        }
    }
}
