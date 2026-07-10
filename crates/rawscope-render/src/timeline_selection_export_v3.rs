//! JSON and Markdown artifacts for timeline selection evidence v3.

use std::collections::HashSet;

use rawscope_core::{RowId, U64Range};
use rawscope_data::{
    DatasetFieldRole, DatasetIdentity, DatasetProfileId, DatasetSource, SyntheticEventType,
    TimelineEventKind, VisualDatasetKind,
};
use serde::Serialize;

use crate::{
    AggregateEvidenceBin, ComparisonRatio, SelectedSourceRowSample, SelectedTimelineEventSampleV2,
    TimelineAggregateEvidenceContext, TimelineAggregateOverview, TimelineEvidenceViewV3,
    TimelineLaneRange, TimelineSelectionComparison, TimelineSelectionEvidenceV3,
};

const MAX_AGGREGATE_CONTEXT_BINS: usize = 16;
const TIMELINE_COMPARISON_BASELINE_LABEL: &str = "active_event_slice";

/// Artifact kind used by timeline selection evidence v3 JSON and export manifests.
pub const TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND: &str = "timeline-selection-evidence";

/// Export error for timeline selection evidence v3 JSON.
pub type TimelineSelectionExportError = serde_json::Error;

/// Serializes timeline selection evidence v3 to deterministic pretty JSON.
pub fn timeline_selection_evidence_v3_json(
    evidence: &TimelineSelectionEvidenceV3,
) -> Result<String, TimelineSelectionExportError> {
    serde_json::to_string_pretty(&TimelineSelectionEvidenceV3Artifact::from(evidence))
}

/// Formats timeline selection evidence v3 as deterministic Markdown.
pub fn timeline_selection_evidence_v3_markdown(evidence: &TimelineSelectionEvidenceV3) -> String {
    let mut markdown = String::new();
    markdown.push_str("# RawScope Timeline Selection Evidence\n\n");
    markdown.push_str(
        "> CPU-side selection evidence with visual encoding, baseline comparison, and bounded aggregate context.\n\n",
    );

    markdown.push_str("## Dataset\n\n");
    markdown.push_str(&format!(
        "- Visual kind: {}\n- Source: {}\n- Dataset row count: {}\n- Field bindings: {}\n{}\n",
        format_visual_dataset_kind(evidence.dataset_identity.visual_kind),
        format_dataset_source(&evidence.dataset_identity.source),
        evidence.dataset_identity.row_count,
        format_field_bindings(&evidence.dataset_identity),
        format_dataset_profile_markdown_line(evidence.active_dataset_profile),
    ));

    markdown.push_str("## View\n\n");
    markdown.push_str(&format!(
        "- Current time range: {}..{}\n- Full time range: {}..{}\n- Lane count: {}\n- Density grid: {}x{}\n- Density encoding: {} | {} | {}\n\n",
        evidence.view.time_range.min,
        evidence.view.time_range.max,
        evidence.view.full_time_range.min,
        evidence.view.full_time_range.max,
        evidence.view.lane_count,
        evidence.view.grid_width,
        evidence.view.grid_height,
        evidence.view.density_encoding.transform.label(),
        evidence.view.density_encoding.palette.label(),
        evidence.view.density_encoding.normalization.label(),
    ));

    markdown.push_str("## Selection\n\n");
    markdown.push_str(&format!(
        "- Selected events: {}\n- Selected percentage: {:.4}%\n- Selected time range: {}..{}\n- Selected lane range: {}..{}\n- Comparison baseline: {} ({} events)\n\n",
        evidence.selected_event_count,
        evidence.selected_percentage,
        evidence.selected_time_range.min,
        evidence.selected_time_range.max,
        evidence.selected_lane_range.start,
        evidence.selected_lane_range.end_exclusive,
        TIMELINE_COMPARISON_BASELINE_LABEL,
        evidence.comparison.baseline_event_count,
    ));

    markdown.push_str("## Event-Kind Comparison\n\n");
    markdown.push_str(
        "| kind | selected_count | baseline_count | selected_% | baseline_% | delta_pp |\n",
    );
    markdown.push_str("| --- | ---: | ---: | ---: | ---: | ---: |\n");
    markdown.push_str(&comparison_ratio_row(
        "background",
        evidence.comparison.event_kind_ratios.background,
    ));
    markdown.push_str(&comparison_ratio_row(
        "spike",
        evidence.comparison.event_kind_ratios.spike,
    ));
    markdown.push_str(&comparison_ratio_row(
        "stale_lane",
        evidence.comparison.event_kind_ratios.stale_lane,
    ));
    markdown.push_str(&comparison_ratio_row(
        "high_value_band",
        evidence.comparison.event_kind_ratios.high_value_band,
    ));
    markdown.push_str(&comparison_ratio_row(
        "unclassified",
        evidence.comparison.event_kind_ratios.unclassified,
    ));
    markdown.push('\n');

    markdown.push_str("## Lane Comparison\n\n");
    markdown.push_str(
        "| lane | selected_count | baseline_count | selected_% | baseline_% | delta_pp |\n",
    );
    markdown.push_str("| ---: | ---: | ---: | ---: | ---: | ---: |\n");
    if evidence.comparison.lane_ratios.is_empty() {
        markdown.push_str("| _none_ |  |  |  |  |  |\n\n");
    } else {
        for (lane, ratio) in evidence.comparison.lane_ratios.iter().copied().enumerate() {
            markdown.push_str(&format!(
                "| {} | {} | {} | {:.4} | {:.4} | {:.4} |\n",
                lane,
                ratio.selected_count,
                ratio.baseline_count,
                ratio.selected_percentage,
                ratio.baseline_percentage,
                ratio.delta_percentage_points,
            ));
        }
        markdown.push('\n');
    }

    markdown.push_str("## Aggregate Context\n\n");
    markdown.push_str(&format!(
        "- Included bins: {} / {}\n\n",
        evidence.aggregate_context.bins.len(),
        evidence.aggregate_context.bin_limit,
    ));
    markdown.push_str("| x_bin | y_bin | count | row_id_sample |\n");
    markdown.push_str("| ---: | ---: | ---: | --- |\n");
    if evidence.aggregate_context.bins.is_empty() {
        markdown.push_str("| _none_ |  |  |  |\n\n");
    } else {
        for bin in &evidence.aggregate_context.bins {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                bin.bin_x,
                bin.bin_y,
                bin.count,
                format_row_id_sample(&bin.row_id_sample),
            ));
        }
        markdown.push('\n');
    }

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
    if evidence.selected_event_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |  |\n\n");
    } else {
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
        markdown.push('\n');
    }

    markdown.push_str("## Sampled Source Rows\n\n");
    if evidence.selected_source_row_sample.is_empty() {
        markdown.push_str("_No retained source rows._\n");
    } else {
        markdown.push_str(&format!(
            "Columns: {}\n\n",
            evidence.selected_source_column_names.join(" | ")
        ));
        for row in &evidence.selected_source_row_sample {
            markdown.push_str(&format!(
                "- Row {}: {}\n",
                row.row_id.0,
                row.values.join(" | ")
            ));
        }
    }

    markdown
}

/// Builds bounded timeline aggregate context for evidence v3.
pub fn timeline_aggregate_evidence_context(
    overview: &TimelineAggregateOverview,
    selected_row_ids: &[RowId],
) -> TimelineAggregateEvidenceContext {
    let selected_row_id_set = selected_row_ids
        .iter()
        .map(|row_id| row_id.0)
        .collect::<HashSet<_>>();
    let mut selected_bins = overview
        .bins
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| {
            let intersects_selected_sample = bin
                .row_ids
                .iter()
                .any(|row_id| selected_row_id_set.contains(&row_id.0));
            intersects_selected_sample.then(|| candidate_bin(index, overview.grid_width, bin))
        })
        .collect::<Vec<_>>();
    selected_bins.sort_by(|left, right| {
        left.bin_y
            .cmp(&right.bin_y)
            .then_with(|| left.bin_x.cmp(&right.bin_x))
    });
    selected_bins.truncate(MAX_AGGREGATE_CONTEXT_BINS);

    let selected_bin_indices = selected_bins
        .iter()
        .map(|candidate| candidate.index)
        .collect::<HashSet<_>>();
    let remaining_capacity = MAX_AGGREGATE_CONTEXT_BINS.saturating_sub(selected_bins.len());

    let mut densest_bins = overview
        .bins
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| {
            let bin_has_context = bin.count > 0 && !selected_bin_indices.contains(&index);
            bin_has_context.then(|| candidate_bin(index, overview.grid_width, bin))
        })
        .collect::<Vec<_>>();
    densest_bins.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.bin_y.cmp(&right.bin_y))
            .then_with(|| left.bin_x.cmp(&right.bin_x))
    });
    densest_bins.truncate(remaining_capacity);

    let bins = selected_bins
        .into_iter()
        .chain(densest_bins)
        .map(AggregateEvidenceBin::from)
        .collect();

    TimelineAggregateEvidenceContext {
        bin_limit: MAX_AGGREGATE_CONTEXT_BINS,
        bins,
    }
}

#[derive(Debug, Serialize)]
struct TimelineSelectionEvidenceV3Artifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_identity: DatasetIdentityArtifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_dataset_profile: Option<String>,
    view: TimelineEvidenceViewV3Artifact,
    selected_event_count: usize,
    selected_percentage: f32,
    selected_time_range: TimeRangeArtifact,
    selected_lane_range: LaneRangeArtifact,
    row_id_sample: Vec<u64>,
    selected_event_sample: Vec<SelectedTimelineEventSampleV2Artifact>,
    source_columns: Vec<String>,
    selected_source_row_sample: Vec<SelectedSourceRowSampleArtifact>,
    comparison: TimelineSelectionComparisonArtifact,
    aggregate_context: TimelineAggregateEvidenceContextArtifact,
}

impl From<&TimelineSelectionEvidenceV3> for TimelineSelectionEvidenceV3Artifact {
    fn from(evidence: &TimelineSelectionEvidenceV3) -> Self {
        Self {
            artifact_kind: TIMELINE_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
            schema_version: evidence.schema_version,
            dataset_identity: DatasetIdentityArtifact::from(&evidence.dataset_identity),
            active_dataset_profile: evidence
                .active_dataset_profile
                .map(format_dataset_profile_id),
            view: TimelineEvidenceViewV3Artifact::from(evidence.view),
            selected_event_count: evidence.selected_event_count,
            selected_percentage: evidence.selected_percentage,
            selected_time_range: TimeRangeArtifact::from(evidence.selected_time_range),
            selected_lane_range: LaneRangeArtifact::from(evidence.selected_lane_range),
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
            comparison: TimelineSelectionComparisonArtifact::from(&evidence.comparison),
            aggregate_context: TimelineAggregateEvidenceContextArtifact::from(
                &evidence.aggregate_context,
            ),
        }
    }
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

#[derive(Debug, Serialize)]
struct TimelineEvidenceViewV3Artifact {
    time_range: TimeRangeArtifact,
    full_time_range: TimeRangeArtifact,
    lane_count: u32,
    grid_width: u32,
    grid_height: u32,
    density_encoding: crate::DensityEncoding,
}

impl From<TimelineEvidenceViewV3> for TimelineEvidenceViewV3Artifact {
    fn from(view: TimelineEvidenceViewV3) -> Self {
        Self {
            time_range: TimeRangeArtifact::from(view.time_range),
            full_time_range: TimeRangeArtifact::from(view.full_time_range),
            lane_count: view.lane_count,
            grid_width: view.grid_width,
            grid_height: view.grid_height,
            density_encoding: view.density_encoding,
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

#[derive(Debug, Serialize)]
struct TimelineSelectionComparisonArtifact {
    baseline: &'static str,
    selected_event_count: usize,
    baseline_event_count: usize,
    selected_percentage: f32,
    event_kind_ratios: TimelineKindComparisonArtifact,
    lane_ratios: Vec<LaneComparisonRatioArtifact>,
}

impl From<&TimelineSelectionComparison> for TimelineSelectionComparisonArtifact {
    fn from(comparison: &TimelineSelectionComparison) -> Self {
        Self {
            baseline: TIMELINE_COMPARISON_BASELINE_LABEL,
            selected_event_count: comparison.selected_event_count,
            baseline_event_count: comparison.baseline_event_count,
            selected_percentage: comparison.selected_percentage,
            event_kind_ratios: TimelineKindComparisonArtifact {
                background: ComparisonRatioArtifact::from(comparison.event_kind_ratios.background),
                spike: ComparisonRatioArtifact::from(comparison.event_kind_ratios.spike),
                stale_lane: ComparisonRatioArtifact::from(comparison.event_kind_ratios.stale_lane),
                high_value_band: ComparisonRatioArtifact::from(
                    comparison.event_kind_ratios.high_value_band,
                ),
                unclassified: ComparisonRatioArtifact::from(
                    comparison.event_kind_ratios.unclassified,
                ),
            },
            lane_ratios: comparison
                .lane_ratios
                .iter()
                .copied()
                .enumerate()
                .map(|(lane, ratio)| LaneComparisonRatioArtifact {
                    lane: lane as u32,
                    ratio: ComparisonRatioArtifact::from(ratio),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct TimelineKindComparisonArtifact {
    background: ComparisonRatioArtifact,
    spike: ComparisonRatioArtifact,
    stale_lane: ComparisonRatioArtifact,
    high_value_band: ComparisonRatioArtifact,
    unclassified: ComparisonRatioArtifact,
}

#[derive(Debug, Serialize)]
struct LaneComparisonRatioArtifact {
    lane: u32,
    #[serde(flatten)]
    ratio: ComparisonRatioArtifact,
}

#[derive(Debug, Serialize)]
struct ComparisonRatioArtifact {
    selected_count: usize,
    baseline_count: usize,
    selected_percentage: f32,
    baseline_percentage: f32,
    delta_percentage_points: f32,
}

impl From<ComparisonRatio> for ComparisonRatioArtifact {
    fn from(ratio: ComparisonRatio) -> Self {
        Self {
            selected_count: ratio.selected_count,
            baseline_count: ratio.baseline_count,
            selected_percentage: ratio.selected_percentage,
            baseline_percentage: ratio.baseline_percentage,
            delta_percentage_points: ratio.delta_percentage_points,
        }
    }
}

#[derive(Debug, Serialize)]
struct TimelineAggregateEvidenceContextArtifact {
    bin_limit: usize,
    bins: Vec<AggregateEvidenceBinArtifact>,
}

impl From<&TimelineAggregateEvidenceContext> for TimelineAggregateEvidenceContextArtifact {
    fn from(context: &TimelineAggregateEvidenceContext) -> Self {
        Self {
            bin_limit: context.bin_limit,
            bins: context
                .bins
                .iter()
                .map(AggregateEvidenceBinArtifact::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct AggregateEvidenceBinArtifact {
    bin_x: u32,
    bin_y: u32,
    count: u32,
    row_id_sample: Vec<u64>,
}

impl From<&AggregateEvidenceBin> for AggregateEvidenceBinArtifact {
    fn from(bin: &AggregateEvidenceBin) -> Self {
        Self {
            bin_x: bin.bin_x,
            bin_y: bin.bin_y,
            count: bin.count,
            row_id_sample: bin.row_id_sample.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CandidateBin<'a> {
    index: usize,
    bin_x: u32,
    bin_y: u32,
    count: u32,
    row_ids: &'a [RowId],
}

impl From<CandidateBin<'_>> for AggregateEvidenceBin {
    fn from(candidate: CandidateBin<'_>) -> Self {
        Self {
            bin_x: candidate.bin_x,
            bin_y: candidate.bin_y,
            count: candidate.count,
            row_id_sample: candidate.row_ids.iter().map(|row_id| row_id.0).collect(),
        }
    }
}

fn candidate_bin<'a>(
    index: usize,
    grid_width: u32,
    bin: &'a crate::AggregateBinSample,
) -> CandidateBin<'a> {
    CandidateBin {
        index,
        bin_x: index as u32 % grid_width,
        bin_y: index as u32 / grid_width,
        count: bin.count,
        row_ids: &bin.row_ids,
    }
}

fn comparison_ratio_row(label: &str, ratio: ComparisonRatio) -> String {
    format!(
        "| {label} | {} | {} | {:.4} | {:.4} | {:.4} |\n",
        ratio.selected_count,
        ratio.baseline_count,
        ratio.selected_percentage,
        ratio.baseline_percentage,
        ratio.delta_percentage_points,
    )
}

fn format_row_id_sample(row_ids: &[u64]) -> String {
    if row_ids.is_empty() {
        return "none".to_string();
    }

    row_ids
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
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
            Some(limit) => format!("local_csv ({}, limit {limit})", path.display()),
            None => format!("local_csv ({})", path.display()),
        },
        DatasetSource::LocalParquet { path, limit } => match limit {
            Some(limit) => format!("local_parquet ({}, limit {limit})", path.display()),
            None => format!("local_parquet ({})", path.display()),
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

fn format_dataset_profile_markdown_line(profile_id: Option<DatasetProfileId>) -> String {
    profile_id
        .map(|profile_id| format!("- Active dataset profile: {}\n", profile_id.as_str()))
        .unwrap_or_default()
}

fn format_dataset_profile_id(profile_id: DatasetProfileId) -> String {
    profile_id.as_str().to_string()
}
