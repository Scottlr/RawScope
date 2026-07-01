//! JSON and Markdown artifacts for timeline selection evidence.

use rawscope_core::U64Range;
use rawscope_data::{SyntheticDatasetMetadata, SyntheticEventType};
use serde::Serialize;

use crate::{
    SelectedEventTypeCounts, SelectedTimelineEventSample, TimelineLaneRange,
    TimelineSelectionEvidence,
};

/// Artifact kind used by timeline selection evidence JSON and export manifests.
pub const TIMELINE_SELECTION_EVIDENCE_ARTIFACT_KIND: &str = "timeline-selection-evidence";

/// Schema version for timeline selection evidence artifacts.
pub const TIMELINE_SELECTION_EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// Serializes timeline selection evidence to deterministic pretty JSON.
pub fn timeline_selection_evidence_json(
    evidence: &TimelineSelectionEvidence,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&TimelineSelectionEvidenceArtifact::from(evidence))
}

/// Formats timeline selection evidence as deterministic Markdown.
pub fn timeline_selection_evidence_markdown(evidence: &TimelineSelectionEvidence) -> String {
    let mut markdown = String::new();
    markdown.push_str("# RawScope Timeline Selection Evidence\n\n");
    markdown.push_str("> Synthetic CPU-side evidence. This is not a benchmark, final report system, or GPU row-id path.\n\n");
    markdown.push_str("## Dataset\n\n");
    markdown.push_str(&format!(
        "- Seed: {}\n- Dataset event count: {}\n- Event preset count: {}\n\n",
        evidence.dataset_metadata.seed, evidence.dataset_metadata.row_count, evidence.event_count,
    ));

    markdown.push_str("## Selected Region\n\n");
    markdown.push_str(&format!(
        "- Selected events: {}\n- Selected percentage: {:.4}%\n- Brush time range: {}..{}\n- Brush lane range: {}..{}\n",
        evidence.selected_event_count,
        evidence.selected_percentage,
        evidence.selected_time_range.min,
        evidence.selected_time_range.max,
        evidence.selected_lane_range.start,
        evidence.selected_lane_range.end_exclusive,
    ));
    markdown.push_str(&format!(
        "- Selected timestamp extent: {}\n- Selected value extent: {}\n\n",
        format_optional_time_range(evidence.selected_timestamp_range),
        format_optional_value_range(evidence.selected_value_range),
    ));

    markdown.push_str("## Lane Counts\n\n");
    if evidence.lane_counts.is_empty() {
        markdown.push_str("_No lanes were available._\n\n");
    } else {
        for (lane, count) in evidence.lane_counts.iter().copied().enumerate() {
            markdown.push_str(&format!("- Lane {lane}: {count}\n"));
        }
        markdown.push_str(&format!(
            "- Top lane: {}\n\n",
            format_optional_u32(evidence.top_lane),
        ));
    }

    markdown.push_str("## Event-Type Counts\n\n");
    markdown.push_str(&format!(
        "- Background: {}\n- Spike: {}\n- Stale lane: {}\n- High-value band: {}\n- Top event type: {}\n\n",
        evidence.event_type_counts.background,
        evidence.event_type_counts.spike,
        evidence.event_type_counts.stale_lane,
        evidence.event_type_counts.high_value_band,
        format_optional_event_type(evidence.top_event_type),
    ));

    markdown.push_str("## Sampled Row IDs\n\n");
    if evidence.selected_row_id_sample.is_empty() {
        markdown.push_str("_No selected row ids._\n\n");
    } else {
        let sampled_row_ids = evidence
            .selected_row_id_sample
            .iter()
            .map(|row_id| row_id.0.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        markdown.push_str(&format!("{sampled_row_ids}\n\n"));
    }

    markdown.push_str("## Sampled Events\n\n");
    markdown.push_str("| row_id | timestamp | lane | value | event_type |\n");
    markdown.push_str("| ---: | ---: | ---: | ---: | --- |\n");
    for sample in &evidence.selected_event_sample {
        markdown.push_str(&format!(
            "| {} | {} | {} | {:.6} | {:?} |\n",
            sample.row_id.0, sample.timestamp, sample.lane, sample.value, sample.event_type,
        ));
    }
    if evidence.selected_event_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |  |\n");
    }

    markdown
}

#[derive(Debug, Serialize)]
struct TimelineSelectionEvidenceArtifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_metadata: DatasetMetadataArtifact,
    event_count: usize,
    selected_time_range: TimeRangeArtifact,
    selected_lane_range: LaneRangeArtifact,
    selected_event_count: usize,
    selected_percentage: f32,
    selected_timestamp_range: Option<TimeRangeArtifact>,
    selected_value_range: Option<ValueRangeArtifact>,
    lane_counts: Vec<usize>,
    event_type_counts: EventTypeCountsArtifact,
    top_lane: Option<u32>,
    top_event_type: Option<String>,
    row_id_sample: Vec<u64>,
    selected_event_sample: Vec<SelectedTimelineEventSampleArtifact>,
}

impl From<&TimelineSelectionEvidence> for TimelineSelectionEvidenceArtifact {
    fn from(evidence: &TimelineSelectionEvidence) -> Self {
        Self {
            artifact_kind: TIMELINE_SELECTION_EVIDENCE_ARTIFACT_KIND,
            schema_version: TIMELINE_SELECTION_EVIDENCE_SCHEMA_VERSION,
            dataset_metadata: DatasetMetadataArtifact::from(&evidence.dataset_metadata),
            event_count: evidence.event_count,
            selected_time_range: TimeRangeArtifact::from(evidence.selected_time_range),
            selected_lane_range: LaneRangeArtifact::from(evidence.selected_lane_range),
            selected_event_count: evidence.selected_event_count,
            selected_percentage: evidence.selected_percentage,
            selected_timestamp_range: evidence
                .selected_timestamp_range
                .map(TimeRangeArtifact::from),
            selected_value_range: evidence.selected_value_range.map(ValueRangeArtifact::from),
            lane_counts: evidence.lane_counts.clone(),
            event_type_counts: EventTypeCountsArtifact::from(evidence.event_type_counts),
            top_lane: evidence.top_lane,
            top_event_type: evidence.top_event_type.map(format_event_type),
            row_id_sample: evidence
                .selected_row_id_sample
                .iter()
                .map(|row_id| row_id.0)
                .collect(),
            selected_event_sample: evidence
                .selected_event_sample
                .iter()
                .map(SelectedTimelineEventSampleArtifact::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct DatasetMetadataArtifact {
    seed: u64,
    row_count: usize,
}

impl From<&SyntheticDatasetMetadata> for DatasetMetadataArtifact {
    fn from(metadata: &SyntheticDatasetMetadata) -> Self {
        Self {
            seed: metadata.seed,
            row_count: metadata.row_count,
        }
    }
}

#[derive(Debug, Serialize)]
struct TimeRangeArtifact {
    min: u64,
    max: u64,
}

impl From<U64Range> for TimeRangeArtifact {
    fn from(range: U64Range) -> Self {
        Self {
            min: range.min,
            max: range.max,
        }
    }
}

#[derive(Debug, Serialize)]
struct LaneRangeArtifact {
    start: u32,
    end_exclusive: u32,
}

impl From<TimelineLaneRange> for LaneRangeArtifact {
    fn from(range: TimelineLaneRange) -> Self {
        Self {
            start: range.start,
            end_exclusive: range.end_exclusive,
        }
    }
}

#[derive(Debug, Serialize)]
struct ValueRangeArtifact {
    min: f32,
    max: f32,
}

impl From<(f32, f32)> for ValueRangeArtifact {
    fn from((min, max): (f32, f32)) -> Self {
        Self { min, max }
    }
}

#[derive(Debug, Serialize)]
struct EventTypeCountsArtifact {
    background: usize,
    spike: usize,
    stale_lane: usize,
    high_value_band: usize,
}

impl From<SelectedEventTypeCounts> for EventTypeCountsArtifact {
    fn from(counts: SelectedEventTypeCounts) -> Self {
        Self {
            background: counts.background,
            spike: counts.spike,
            stale_lane: counts.stale_lane,
            high_value_band: counts.high_value_band,
        }
    }
}

#[derive(Debug, Serialize)]
struct SelectedTimelineEventSampleArtifact {
    row_id: u64,
    timestamp: u64,
    lane: u32,
    value: f32,
    event_type: String,
}

impl From<&SelectedTimelineEventSample> for SelectedTimelineEventSampleArtifact {
    fn from(sample: &SelectedTimelineEventSample) -> Self {
        Self {
            row_id: sample.row_id.0,
            timestamp: sample.timestamp,
            lane: sample.lane,
            value: sample.value,
            event_type: format_event_type(sample.event_type),
        }
    }
}

fn format_optional_time_range(range: Option<U64Range>) -> String {
    range
        .map(|range| format!("{}..{}", range.min, range.max))
        .unwrap_or_else(|| "none".to_string())
}

fn format_optional_value_range(range: Option<(f32, f32)>) -> String {
    range
        .map(|(min, max)| format!("{min:.6}..{max:.6}"))
        .unwrap_or_else(|| "none".to_string())
}

fn format_optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn format_optional_event_type(event_type: Option<SyntheticEventType>) -> String {
    event_type
        .map(format_event_type)
        .unwrap_or_else(|| "none".to_string())
}

fn format_event_type(event_type: SyntheticEventType) -> String {
    format!("{event_type:?}")
}
