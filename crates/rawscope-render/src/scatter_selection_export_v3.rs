//! JSON and Markdown artifacts for scatter selection evidence v3.

use std::collections::HashSet;

use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    DatasetFieldRole, DatasetIdentity, DatasetProfileId, DatasetSource, ScatterPointKind,
    SyntheticPointCategory, VisualDatasetKind,
};
use serde::Serialize;

use crate::{
    AggregateEvidenceBin, ComparisonRatio, ScatterAggregateEvidenceContext,
    ScatterAggregateOverview, ScatterEvidenceViewV3, ScatterSelectionComparison,
    ScatterSelectionEvidenceV3,
};
use rawscope_evidence::{SelectedPointSampleV2, SelectedSourceRowSample};

const MAX_AGGREGATE_CONTEXT_BINS: usize = 16;
const SCATTER_COMPARISON_BASELINE_LABEL: &str = "active_point_slice";

/// Artifact kind used by scatter selection evidence v3 JSON and export manifests.
pub const SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND: &str = "scatter-selection-evidence";

/// Export error for scatter selection evidence v3 JSON.
pub type ScatterSelectionExportError = serde_json::Error;

/// Serializes scatter selection evidence v3 to deterministic pretty JSON.
pub fn scatter_selection_evidence_v3_json(
    evidence: &ScatterSelectionEvidenceV3,
) -> Result<String, ScatterSelectionExportError> {
    serde_json::to_string_pretty(&ScatterSelectionEvidenceV3Artifact::from(evidence))
}

/// Formats scatter selection evidence v3 as deterministic Markdown.
pub fn scatter_selection_evidence_v3_markdown(evidence: &ScatterSelectionEvidenceV3) -> String {
    let mut markdown = String::new();
    markdown.push_str("# RawScope Scatter Selection Evidence\n\n");
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
        "- Current x range: {:.6}..{:.6}\n- Current y range: {:.6}..{:.6}\n- Density grid: {}x{}\n- Density presentation: {}\n- Density encoding: {} | {} | {}\n\n",
        evidence.view.x_range.min,
        evidence.view.x_range.max,
        evidence.view.y_range.min,
        evidence.view.y_range.max,
        evidence.view.grid_width,
        evidence.view.grid_height,
        evidence.view.density_presentation.evidence_label(),
        evidence.view.density_encoding.transform.label(),
        evidence.view.density_encoding.palette.label(),
        evidence.view.density_encoding.normalization.label(),
    ));

    markdown.push_str("## Selection\n\n");
    markdown.push_str(&format!(
        "- Selected rows: {}\n- Selected percentage: {:.4}%\n- Comparison baseline: {} ({} rows)\n\n",
        evidence.selected_row_count,
        evidence.selected_percentage,
        SCATTER_COMPARISON_BASELINE_LABEL,
        evidence.comparison.baseline_row_count,
    ));

    markdown.push_str("## Comparison\n\n");
    markdown.push_str(
        "| kind | selected_count | baseline_count | selected_% | baseline_% | delta_pp |\n",
    );
    markdown.push_str("| --- | ---: | ---: | ---: | ---: | ---: |\n");
    markdown.push_str(&comparison_ratio_row(
        "cluster",
        evidence.comparison.point_kind_ratios.cluster,
    ));
    markdown.push_str(&comparison_ratio_row(
        "background",
        evidence.comparison.point_kind_ratios.background,
    ));
    markdown.push_str(&comparison_ratio_row(
        "outlier",
        evidence.comparison.point_kind_ratios.outlier,
    ));
    markdown.push_str(&comparison_ratio_row(
        "unclassified",
        evidence.comparison.point_kind_ratios.unclassified,
    ));
    markdown.push('\n');

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

    markdown.push_str("## Sampled Visual Records\n\n");
    markdown.push_str("| row_id | x | y | kind |\n");
    markdown.push_str("| ---: | ---: | ---: | --- |\n");
    if evidence.selected_record_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |\n\n");
    } else {
        for sample in &evidence.selected_record_sample {
            markdown.push_str(&format!(
                "| {} | {:.6} | {:.6} | {} |\n",
                sample.row_id.0,
                sample.x,
                sample.y,
                format_scatter_point_kind(sample.kind),
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

/// Builds bounded scatter aggregate context for evidence v3.
pub fn scatter_aggregate_evidence_context(
    overview: &ScatterAggregateOverview,
    selected_row_ids: &[RowId],
) -> ScatterAggregateEvidenceContext {
    let selected_row_id_set = selected_row_ids
        .iter()
        .map(|row_id| row_id.0)
        .collect::<HashSet<_>>();
    let selected_bin_indices = overview
        .bins
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| {
            let intersects_selected_sample = bin
                .row_ids
                .iter()
                .any(|row_id| selected_row_id_set.contains(&row_id.0));
            intersects_selected_sample.then_some(index)
        })
        .collect::<Vec<_>>();
    scatter_aggregate_evidence_context_for_bins(overview, &selected_bin_indices)
}

/// Builds aggregate context from complete selected-bin membership.
pub fn scatter_aggregate_evidence_context_for_bins(
    overview: &ScatterAggregateOverview,
    selected_bin_indices: &[usize],
) -> ScatterAggregateEvidenceContext {
    let selected_bin_index_set = selected_bin_indices.iter().copied().collect::<HashSet<_>>();
    let mut selected_bins = overview
        .bins
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| {
            selected_bin_index_set
                .contains(&index)
                .then(|| candidate_bin(index, overview.grid_width, bin))
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
        .map(|candidate| AggregateEvidenceBin {
            bin_x: candidate.bin_x,
            bin_y: candidate.bin_y,
            count: candidate.count,
            row_id_sample: candidate.row_ids.iter().map(|row_id| row_id.0).collect(),
        })
        .collect();

    ScatterAggregateEvidenceContext {
        bin_limit: MAX_AGGREGATE_CONTEXT_BINS,
        bins,
    }
}

#[derive(Debug, Serialize)]
struct ScatterSelectionEvidenceV3Artifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_identity: DatasetIdentityArtifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_dataset_profile: Option<String>,
    view: ScatterEvidenceViewV3Artifact,
    selected_row_count: usize,
    selected_percentage: f32,
    row_id_sample: Vec<u64>,
    selected_record_sample: Vec<SelectedPointSampleV2Artifact>,
    source_columns: Vec<String>,
    selected_source_row_sample: Vec<SelectedSourceRowSampleArtifact>,
    comparison: ScatterSelectionComparisonArtifact,
    aggregate_context: ScatterAggregateEvidenceContextArtifact,
}

impl From<&ScatterSelectionEvidenceV3> for ScatterSelectionEvidenceV3Artifact {
    fn from(evidence: &ScatterSelectionEvidenceV3) -> Self {
        Self {
            artifact_kind: SCATTER_SELECTION_EVIDENCE_V3_ARTIFACT_KIND,
            schema_version: evidence.schema_version,
            dataset_identity: DatasetIdentityArtifact::from(&evidence.dataset_identity),
            active_dataset_profile: evidence
                .active_dataset_profile
                .map(format_dataset_profile_id),
            view: ScatterEvidenceViewV3Artifact::from(evidence.view),
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            row_id_sample: evidence
                .selected_row_id_sample
                .iter()
                .map(|row_id| row_id.0)
                .collect(),
            selected_record_sample: evidence
                .selected_record_sample
                .iter()
                .map(SelectedPointSampleV2Artifact::from)
                .collect(),
            source_columns: evidence.selected_source_column_names.clone(),
            selected_source_row_sample: evidence
                .selected_source_row_sample
                .iter()
                .map(SelectedSourceRowSampleArtifact::from)
                .collect(),
            comparison: ScatterSelectionComparisonArtifact::from(&evidence.comparison),
            aggregate_context: ScatterAggregateEvidenceContextArtifact::from(
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
struct ScatterEvidenceViewV3Artifact {
    x_range: RangeArtifact,
    y_range: RangeArtifact,
    grid_width: u32,
    grid_height: u32,
    density_encoding: crate::DensityEncoding,
    density_presentation: crate::ScatterDensityPresentation,
}

impl From<ScatterEvidenceViewV3> for ScatterEvidenceViewV3Artifact {
    fn from(view: ScatterEvidenceViewV3) -> Self {
        Self {
            x_range: RangeArtifact::from(view.x_range),
            y_range: RangeArtifact::from(view.y_range),
            grid_width: view.grid_width,
            grid_height: view.grid_height,
            density_encoding: view.density_encoding,
            density_presentation: view.density_presentation,
        }
    }
}

#[derive(Debug, Serialize)]
struct RangeArtifact {
    min: f32,
    max: f32,
}

impl From<F32Range> for RangeArtifact {
    fn from(range: F32Range) -> Self {
        Self {
            min: range.min,
            max: range.max,
        }
    }
}

#[derive(Debug, Serialize)]
struct SelectedPointSampleV2Artifact {
    row_id: u64,
    x: f32,
    y: f32,
    kind: String,
}

impl From<&SelectedPointSampleV2> for SelectedPointSampleV2Artifact {
    fn from(sample: &SelectedPointSampleV2) -> Self {
        Self {
            row_id: sample.row_id.0,
            x: sample.x,
            y: sample.y,
            kind: format_scatter_point_kind(sample.kind),
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
struct ScatterSelectionComparisonArtifact {
    baseline: &'static str,
    selected_row_count: usize,
    baseline_row_count: usize,
    selected_percentage: f32,
    point_kind_ratios: ScatterKindComparisonArtifact,
}

impl From<&ScatterSelectionComparison> for ScatterSelectionComparisonArtifact {
    fn from(comparison: &ScatterSelectionComparison) -> Self {
        Self {
            baseline: SCATTER_COMPARISON_BASELINE_LABEL,
            selected_row_count: comparison.selected_row_count,
            baseline_row_count: comparison.baseline_row_count,
            selected_percentage: comparison.selected_percentage,
            point_kind_ratios: ScatterKindComparisonArtifact {
                cluster: ComparisonRatioArtifact::from(comparison.point_kind_ratios.cluster),
                background: ComparisonRatioArtifact::from(comparison.point_kind_ratios.background),
                outlier: ComparisonRatioArtifact::from(comparison.point_kind_ratios.outlier),
                unclassified: ComparisonRatioArtifact::from(
                    comparison.point_kind_ratios.unclassified,
                ),
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct ScatterKindComparisonArtifact {
    cluster: ComparisonRatioArtifact,
    background: ComparisonRatioArtifact,
    outlier: ComparisonRatioArtifact,
    unclassified: ComparisonRatioArtifact,
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
struct ScatterAggregateEvidenceContextArtifact {
    bin_limit: usize,
    bins: Vec<AggregateEvidenceBinArtifact>,
}

impl From<&ScatterAggregateEvidenceContext> for ScatterAggregateEvidenceContextArtifact {
    fn from(context: &ScatterAggregateEvidenceContext) -> Self {
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

fn format_dataset_profile_markdown_line(profile_id: Option<DatasetProfileId>) -> String {
    profile_id
        .map(|profile_id| format!("- Active dataset profile: {}\n", profile_id.as_str()))
        .unwrap_or_default()
}

fn format_dataset_profile_id(profile_id: DatasetProfileId) -> String {
    profile_id.as_str().to_string()
}

fn format_scatter_point_kind(kind: ScatterPointKind) -> String {
    match kind {
        ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster) => "cluster".to_string(),
        ScatterPointKind::Synthetic(SyntheticPointCategory::Background) => "background".to_string(),
        ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier) => "outlier".to_string(),
        ScatterPointKind::Unclassified => "unclassified".to_string(),
    }
}
