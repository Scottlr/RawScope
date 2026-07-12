//! JSON and Markdown artifacts for timeline selection evidence.

use rawscope_core::U64Range;
use rawscope_data::{
    DatasetFieldRole, DatasetIdentity, DatasetSource, SyntheticDatasetMetadata, SyntheticEventType,
    TimelineEventKind, VisualDatasetKind,
};
use serde::Serialize;

use crate::{
    SelectedEventTypeCounts, SelectedTimelineEventSample, SelectedTimelineEventSampleV2,
    TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidence,
    TimelineSelectionEvidenceV2,
};
use rawscope_evidence::SelectedSourceRowSample;
use rawscope_evidence::{escape_table_cell, portable_path_label};

/// Artifact kind used by timeline selection evidence JSON and export manifests.
pub const TIMELINE_SELECTION_EVIDENCE_ARTIFACT_KIND: &str = "timeline-selection-evidence";

/// Schema version for timeline selection evidence artifacts.
pub const TIMELINE_SELECTION_EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// Artifact kind used by timeline selection evidence v2 JSON and export manifests.
pub const TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND: &str = "timeline-selection-evidence";

/// Schema version for timeline selection evidence v2 artifacts.
pub const TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION: u32 = 2;

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
            "| {} | {} | {} | {:.6} | {} |\n",
            sample.row_id.0,
            sample.timestamp,
            sample.lane,
            sample.value,
            format_optional_event_type(sample.event_type),
        ));
    }
    if evidence.selected_event_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |  |\n");
    }

    markdown
}

/// Serializes timeline selection evidence v2 to deterministic pretty JSON.
pub fn timeline_selection_evidence_v2_json(
    evidence: &TimelineSelectionEvidenceV2,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&TimelineSelectionEvidenceV2Artifact::from(evidence))
}

/// Formats timeline selection evidence v2 as deterministic Markdown.
pub fn timeline_selection_evidence_v2_markdown(evidence: &TimelineSelectionEvidenceV2) -> String {
    let mut markdown = String::new();
    markdown.push_str("# RawScope Timeline Selection Evidence\n\n");
    markdown.push_str(
        "> CPU-side selection evidence. This is not a benchmark, final report system, or GPU row-id path.\n\n",
    );
    markdown.push_str("## Dataset\n\n");
    markdown.push_str(&format!(
        "- Visual kind: {}\n- Source: {}\n- Dataset row count: {}\n- Field bindings: {}\n",
        format_visual_dataset_kind(evidence.dataset_identity.visual_kind),
        format_dataset_source(&evidence.dataset_identity.source),
        evidence.dataset_identity.row_count,
        format_field_bindings(&evidence.dataset_identity),
    ));
    if !evidence.dataset_identity.lane_labels.is_empty() {
        markdown.push_str(&format!(
            "- Lane labels: {}\n",
            evidence.dataset_identity.lane_labels.join(", ")
        ));
    }
    markdown.push('\n');

    markdown.push_str("## View\n\n");
    markdown.push_str(&format!(
        "- Current time range: {}..{}\n- Full time range: {}..{}\n- Lane count: {}\n- Density grid: {}x{}\n\n",
        evidence.view.time_range.min,
        evidence.view.time_range.max,
        evidence.view.full_time_range.min,
        evidence.view.full_time_range.max,
        evidence.view.lane_count,
        evidence.view.grid_width,
        evidence.view.grid_height,
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

    markdown.push_str("## Event-Kind Counts\n\n");
    markdown.push_str(&format!(
        "- Background: {}\n- Spike: {}\n- Stale lane: {}\n- High-value band: {}\n- Unclassified: {}\n- Top event kind: {}\n\n",
        evidence.event_kind_counts.background,
        evidence.event_kind_counts.spike,
        evidence.event_kind_counts.stale_lane,
        evidence.event_kind_counts.high_value_band,
        evidence.event_kind_counts.unclassified,
        evidence
            .top_event_kind
            .map(format_timeline_event_kind)
            .unwrap_or_else(|| "none".to_string()),
    ));

    markdown.push_str("## Sampled Row IDs\n\n");
    if evidence.selected_row_id_sample.is_empty() {
        markdown.push_str("_No selected row ids._\n\n");
    } else {
        markdown.push_str(&format!(
            "{}\n\n",
            evidence
                .selected_row_id_sample
                .iter()
                .map(|row_id| row_id.0.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    markdown.push_str("## Sampled Visual Events\n\n");
    markdown.push_str("| row_id | timestamp | lane | value | kind |\n");
    markdown.push_str("| ---: | ---: | ---: | ---: | --- |\n");
    for sample in &evidence.selected_event_sample {
        markdown.push_str(&format!(
            "| {} | {} | {} | {:.6} | {} |\n",
            sample.row_id.0,
            sample.timestamp,
            sample.lane,
            sample.value,
            format_timeline_event_kind(sample.kind),
        ));
    }
    if evidence.selected_event_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |  |\n");
    }
    markdown.push('\n');

    markdown.push_str("## Sampled Source Rows\n\n");
    if evidence.selected_source_row_sample.is_empty() {
        markdown.push_str("_No retained source rows._\n");
    } else {
        markdown.push_str(&format!(
            "Columns: {}\n\n",
            evidence
                .selected_source_column_names
                .iter()
                .map(|value| escape_table_cell(value))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        for row in &evidence.selected_source_row_sample {
            markdown.push_str(&format!(
                "- Row {}: {}\n",
                row.row_id.0,
                row.values
                    .iter()
                    .map(|value| escape_table_cell(value))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
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

#[derive(Debug, Serialize)]
struct TimelineSelectionEvidenceV2Artifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_identity: DatasetIdentityArtifact,
    view: TimelineEvidenceViewArtifact,
    brush_range: TimelineBrushRangeArtifact,
    selected_event_count: usize,
    selected_percentage: f32,
    selected_timestamp_range: Option<TimeRangeArtifact>,
    selected_value_range: Option<ValueRangeArtifact>,
    lane_counts: Vec<usize>,
    event_kind_counts: EventKindCountsArtifact,
    top_lane: Option<u32>,
    top_event_kind: Option<String>,
    row_id_sample: Vec<u64>,
    selected_event_sample: Vec<SelectedTimelineEventSampleV2Artifact>,
    source_columns: Vec<String>,
    selected_source_row_sample: Vec<SelectedSourceRowSampleArtifact>,
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

impl From<&TimelineSelectionEvidenceV2> for TimelineSelectionEvidenceV2Artifact {
    fn from(evidence: &TimelineSelectionEvidenceV2) -> Self {
        Self {
            artifact_kind: TIMELINE_SELECTION_EVIDENCE_V2_ARTIFACT_KIND,
            schema_version: TIMELINE_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
            dataset_identity: DatasetIdentityArtifact::from(&evidence.dataset_identity),
            view: TimelineEvidenceViewArtifact::from(evidence.view),
            brush_range: TimelineBrushRangeArtifact::from((
                evidence.selected_time_range,
                evidence.selected_lane_range,
            )),
            selected_event_count: evidence.selected_event_count,
            selected_percentage: evidence.selected_percentage,
            selected_timestamp_range: evidence
                .selected_timestamp_range
                .map(TimeRangeArtifact::from),
            selected_value_range: evidence.selected_value_range.map(ValueRangeArtifact::from),
            lane_counts: evidence.lane_counts.clone(),
            event_kind_counts: EventKindCountsArtifact::from(evidence.event_kind_counts),
            top_lane: evidence.top_lane,
            top_event_kind: evidence.top_event_kind.map(format_timeline_event_kind),
            row_id_sample: evidence
                .selected_row_id_sample
                .iter()
                .map(|row_id| row_id.0)
                .collect(),
            selected_event_sample: evidence
                .selected_event_sample
                .iter()
                .map(SelectedTimelineEventSampleV2Artifact::from)
                .collect(),
            source_columns: evidence.selected_source_column_names.clone(),
            selected_source_row_sample: evidence
                .selected_source_row_sample
                .iter()
                .map(SelectedSourceRowSampleArtifact::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct DatasetMetadataArtifact {
    seed: u64,
    row_count: usize,
}

#[derive(Debug, Serialize)]
struct DatasetIdentityArtifact {
    visual_kind: String,
    source: DatasetSourceArtifact,
    row_count: usize,
    field_bindings: Vec<FieldBindingArtifact>,
    lane_labels: Vec<String>,
}

impl From<&DatasetIdentity> for DatasetIdentityArtifact {
    fn from(identity: &DatasetIdentity) -> Self {
        Self {
            visual_kind: format_visual_dataset_kind(identity.visual_kind),
            source: DatasetSourceArtifact::from(&identity.source),
            row_count: identity.row_count,
            field_bindings: identity
                .field_bindings
                .iter()
                .map(FieldBindingArtifact::from)
                .collect(),
            lane_labels: identity.lane_labels.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DatasetSourceArtifact {
    Synthetic { seed: u64, generator: &'static str },
    LocalCsv { path: String, limit: Option<usize> },
    LocalParquet { path: String, limit: Option<usize> },
}

impl From<&DatasetSource> for DatasetSourceArtifact {
    fn from(source: &DatasetSource) -> Self {
        match source {
            DatasetSource::Synthetic { seed, generator } => Self::Synthetic {
                seed: *seed,
                generator,
            },
            DatasetSource::LocalCsv { path, limit } => Self::LocalCsv {
                path: path.display().to_string(),
                limit: *limit,
            },
            DatasetSource::LocalParquet { path, limit } => Self::LocalParquet {
                path: path.display().to_string(),
                limit: *limit,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct FieldBindingArtifact {
    role: String,
    column_name: String,
}

impl From<&rawscope_data::DatasetFieldBinding> for FieldBindingArtifact {
    fn from(binding: &rawscope_data::DatasetFieldBinding) -> Self {
        Self {
            role: format_dataset_field_role(binding.role),
            column_name: binding.column_name.clone(),
        }
    }
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

#[derive(Debug, Serialize)]
struct TimelineBrushRangeArtifact {
    time: TimeRangeArtifact,
    lane: LaneRangeArtifact,
}

impl From<(U64Range, TimelineLaneRange)> for TimelineBrushRangeArtifact {
    fn from((time_range, lane_range): (U64Range, TimelineLaneRange)) -> Self {
        Self {
            time: TimeRangeArtifact::from(time_range),
            lane: LaneRangeArtifact::from(lane_range),
        }
    }
}

#[derive(Debug, Serialize)]
struct TimelineEvidenceViewArtifact {
    time_range: TimeRangeArtifact,
    full_time_range: TimeRangeArtifact,
    lane_count: u32,
    grid_width: u32,
    grid_height: u32,
}

impl From<TimelineEvidenceView> for TimelineEvidenceViewArtifact {
    fn from(view: TimelineEvidenceView) -> Self {
        Self {
            time_range: TimeRangeArtifact::from(view.time_range),
            full_time_range: TimeRangeArtifact::from(view.full_time_range),
            lane_count: view.lane_count,
            grid_width: view.grid_width,
            grid_height: view.grid_height,
        }
    }
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

#[derive(Debug, Serialize)]
struct EventKindCountsArtifact {
    background: usize,
    spike: usize,
    stale_lane: usize,
    high_value_band: usize,
    unclassified: usize,
}

impl From<SelectedEventTypeCounts> for EventKindCountsArtifact {
    fn from(counts: SelectedEventTypeCounts) -> Self {
        Self {
            background: counts.background,
            spike: counts.spike,
            stale_lane: counts.stale_lane,
            high_value_band: counts.high_value_band,
            unclassified: counts.unclassified,
        }
    }
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

#[derive(Debug, Serialize)]
struct SelectedTimelineEventSampleV2Artifact {
    row_id: u64,
    timestamp: u64,
    lane: u32,
    value: f32,
    kind: String,
}

impl From<&SelectedTimelineEventSampleV2> for SelectedTimelineEventSampleV2Artifact {
    fn from(sample: &SelectedTimelineEventSampleV2) -> Self {
        Self {
            row_id: sample.row_id.0,
            timestamp: sample.timestamp,
            lane: sample.lane,
            value: sample.value,
            kind: format_timeline_event_kind(sample.kind),
        }
    }
}

#[derive(Debug, Serialize)]
struct SelectedSourceRowSampleArtifact {
    row_id: u64,
    values: Vec<String>,
}

impl From<&SelectedSourceRowSample> for SelectedSourceRowSampleArtifact {
    fn from(sample: &SelectedSourceRowSample) -> Self {
        Self {
            row_id: sample.row_id.0,
            values: sample.values.clone(),
        }
    }
}

impl From<&SelectedTimelineEventSample> for SelectedTimelineEventSampleArtifact {
    fn from(sample: &SelectedTimelineEventSample) -> Self {
        Self {
            row_id: sample.row_id.0,
            timestamp: sample.timestamp,
            lane: sample.lane,
            value: sample.value,
            event_type: format_optional_event_type(sample.event_type),
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

fn format_visual_dataset_kind(kind: VisualDatasetKind) -> String {
    match kind {
        VisualDatasetKind::Scatter => "scatter".to_string(),
        VisualDatasetKind::Timeline => "timeline".to_string(),
    }
}

fn format_dataset_field_role(role: DatasetFieldRole) -> String {
    match role {
        DatasetFieldRole::X => "x".to_string(),
        DatasetFieldRole::Y => "y".to_string(),
        DatasetFieldRole::Time => "time".to_string(),
        DatasetFieldRole::Lane => "lane".to_string(),
    }
}

fn format_field_bindings(identity: &DatasetIdentity) -> String {
    identity
        .field_bindings
        .iter()
        .map(|binding| {
            format!(
                "{}={}",
                format_dataset_field_role(binding.role),
                binding.column_name
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_dataset_source(source: &DatasetSource) -> String {
    match source {
        DatasetSource::Synthetic { seed, generator } => {
            format!("synthetic (seed {seed}, generator {generator})")
        }
        DatasetSource::LocalCsv { path, limit } => match limit {
            Some(limit) => format!("local_csv ({}, limit {limit})", portable_path_label(path)),
            None => format!("local_csv ({})", portable_path_label(path)),
        },
        DatasetSource::LocalParquet { path, limit } => match limit {
            Some(limit) => format!(
                "local_parquet ({}, limit {limit})",
                portable_path_label(path)
            ),
            None => format!("local_parquet ({})", portable_path_label(path)),
        },
    }
}

fn format_timeline_event_kind(kind: TimelineEventKind) -> String {
    match kind {
        TimelineEventKind::Synthetic(SyntheticEventType::Background) => "background".to_string(),
        TimelineEventKind::Synthetic(SyntheticEventType::Spike) => "spike".to_string(),
        TimelineEventKind::Synthetic(SyntheticEventType::StaleLane) => "stale_lane".to_string(),
        TimelineEventKind::Synthetic(SyntheticEventType::HighValueBand) => {
            "high_value_band".to_string()
        }
        TimelineEventKind::Unclassified => "unclassified".to_string(),
    }
}
