//! JSON and Markdown artifacts for scatter selection evidence.

use rawscope_core::F32Range;
use rawscope_data::{
    DatasetFieldRole, DatasetIdentity, DatasetSource, ScatterPointKind, SyntheticDatasetMetadata,
    SyntheticPointCategory, VisualDatasetKind,
};
use serde::Serialize;

use crate::markdown_escape::{escape_table_cell, portable_path_label};
use crate::{
    ScatterEvidenceView, ScatterSelectionEvidence, ScatterSelectionEvidenceV2,
    ScatterSelectionKindCounts, SelectedPointSample, SelectedPointSampleV2,
    SelectedSourceRowSample,
};

/// Artifact kind used by scatter selection evidence JSON and export manifests.
pub const SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND: &str = "scatter-selection-evidence";

/// Schema version for scatter selection evidence artifacts.
pub const SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// Artifact kind used by scatter selection evidence v2 JSON and export manifests.
pub const SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND: &str = "scatter-selection-evidence";

/// Schema version for scatter selection evidence v2 artifacts.
pub const SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION: u32 = 2;

/// Serializes scatter selection evidence to deterministic pretty JSON.
pub fn scatter_selection_evidence_json(
    evidence: &ScatterSelectionEvidence,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&ScatterSelectionEvidenceArtifact::from(evidence))
}

/// Formats scatter selection evidence as deterministic Markdown.
pub fn scatter_selection_evidence_markdown(evidence: &ScatterSelectionEvidence) -> String {
    let mut markdown = String::new();
    markdown.push_str("# RawScope Scatter Selection Evidence\n\n");
    markdown.push_str("> Synthetic CPU-side evidence. This is not a benchmark, final report system, or GPU row-id path.\n\n");
    markdown.push_str("## Dataset\n\n");
    markdown.push_str(&format!(
        "- Seed: {}\n- Dataset row count: {}\n- Point preset row count: {}\n\n",
        evidence.dataset_metadata.seed,
        evidence.dataset_metadata.row_count,
        evidence.point_preset_row_count,
    ));

    markdown.push_str("## Selected Region\n\n");
    markdown.push_str(&format!(
        "- Selected rows: {}\n- Selected percentage: {:.4}%\n- Brush x range: {:.6}..{:.6}\n- Brush y range: {:.6}..{:.6}\n",
        evidence.selected_row_count,
        evidence.selected_percentage,
        evidence.brush_x_range.min,
        evidence.brush_x_range.max,
        evidence.brush_y_range.min,
        evidence.brush_y_range.max,
    ));
    markdown.push_str(&format!(
        "- Selected x extent: {}\n- Selected y extent: {}\n\n",
        format_optional_range(evidence.selected_x_range),
        format_optional_range(evidence.selected_y_range),
    ));

    markdown.push_str("## Category Counts\n\n");
    markdown.push_str(&format!(
        "- Cluster: {}\n- Background: {}\n- Outlier: {}\n- Top category: {}\n\n",
        evidence.category_counts.cluster,
        evidence.category_counts.background,
        evidence.category_counts.outlier,
        format_optional_category(evidence.top_category),
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

    markdown.push_str("## Sampled Records\n\n");
    markdown.push_str("| row_id | x | y | category |\n");
    markdown.push_str("| ---: | ---: | ---: | --- |\n");
    for sample in &evidence.selected_record_sample {
        markdown.push_str(&format!(
            "| {} | {:.6} | {:.6} | {} |\n",
            sample.row_id.0,
            sample.x,
            sample.y,
            format_optional_category(sample.category),
        ));
    }
    if evidence.selected_record_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |\n");
    }

    markdown
}

/// Serializes scatter selection evidence v2 to deterministic pretty JSON.
pub fn scatter_selection_evidence_v2_json(
    evidence: &ScatterSelectionEvidenceV2,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&ScatterSelectionEvidenceV2Artifact::from(evidence))
}

/// Formats scatter selection evidence v2 as deterministic Markdown.
pub fn scatter_selection_evidence_v2_markdown(evidence: &ScatterSelectionEvidenceV2) -> String {
    let mut markdown = String::new();
    markdown.push_str("# RawScope Scatter Selection Evidence\n\n");
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
        "- Current x range: {:.6}..{:.6}\n- Current y range: {:.6}..{:.6}\n- Density grid: {}x{}\n\n",
        evidence.view.x_range.min,
        evidence.view.x_range.max,
        evidence.view.y_range.min,
        evidence.view.y_range.max,
        evidence.view.grid_width,
        evidence.view.grid_height,
    ));

    markdown.push_str("## Selected Region\n\n");
    markdown.push_str(&format!(
        "- Selected rows: {}\n- Selected percentage: {:.4}%\n- Brush x range: {:.6}..{:.6}\n- Brush y range: {:.6}..{:.6}\n",
        evidence.selected_row_count,
        evidence.selected_percentage,
        evidence.brush_x_range.min,
        evidence.brush_x_range.max,
        evidence.brush_y_range.min,
        evidence.brush_y_range.max,
    ));
    markdown.push_str(&format!(
        "- Selected x extent: {}\n- Selected y extent: {}\n\n",
        format_optional_range(evidence.selected_x_range),
        format_optional_range(evidence.selected_y_range),
    ));

    markdown.push_str("## Point-Kind Counts\n\n");
    markdown.push_str(&format!(
        "- Cluster: {}\n- Background: {}\n- Outlier: {}\n- Unclassified: {}\n- Top point kind: {}\n\n",
        evidence.point_kind_counts.cluster,
        evidence.point_kind_counts.background,
        evidence.point_kind_counts.outlier,
        evidence.point_kind_counts.unclassified,
        evidence
            .top_point_kind
            .map(format_scatter_point_kind)
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

    markdown.push_str("## Sampled Visual Records\n\n");
    markdown.push_str("| row_id | x | y | kind |\n");
    markdown.push_str("| ---: | ---: | ---: | --- |\n");
    for sample in &evidence.selected_record_sample {
        markdown.push_str(&format!(
            "| {} | {:.6} | {:.6} | {} |\n",
            sample.row_id.0,
            sample.x,
            sample.y,
            format_scatter_point_kind(sample.kind),
        ));
    }
    if evidence.selected_record_sample.is_empty() {
        markdown.push_str("| _none_ |  |  |  |\n");
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
struct ScatterSelectionEvidenceArtifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_metadata: DatasetMetadataArtifact,
    point_preset_row_count: usize,
    brush_range: BrushRangeArtifact,
    selected_row_count: usize,
    selected_percentage: f32,
    selected_extent: SelectedExtentArtifact,
    category_counts: CategoryCountsArtifact,
    top_category: Option<String>,
    row_id_sample: Vec<u64>,
    selected_record_sample: Vec<SelectedPointSampleArtifact>,
}

#[derive(Debug, Serialize)]
struct ScatterSelectionEvidenceV2Artifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_identity: DatasetIdentityArtifact,
    view: ScatterEvidenceViewArtifact,
    brush_range: BrushRangeArtifact,
    selected_row_count: usize,
    selected_percentage: f32,
    selected_extent: SelectedExtentArtifact,
    point_kind_counts: PointKindCountsArtifact,
    top_point_kind: Option<String>,
    row_id_sample: Vec<u64>,
    selected_record_sample: Vec<SelectedPointSampleV2Artifact>,
    source_columns: Vec<String>,
    selected_source_row_sample: Vec<SelectedSourceRowSampleArtifact>,
}

impl From<&ScatterSelectionEvidence> for ScatterSelectionEvidenceArtifact {
    fn from(evidence: &ScatterSelectionEvidence) -> Self {
        Self {
            artifact_kind: SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND,
            schema_version: SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION,
            dataset_metadata: DatasetMetadataArtifact::from(&evidence.dataset_metadata),
            point_preset_row_count: evidence.point_preset_row_count,
            brush_range: BrushRangeArtifact {
                x: RangeArtifact::from(evidence.brush_x_range),
                y: RangeArtifact::from(evidence.brush_y_range),
            },
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            selected_extent: SelectedExtentArtifact {
                x: evidence.selected_x_range.map(RangeArtifact::from),
                y: evidence.selected_y_range.map(RangeArtifact::from),
            },
            category_counts: CategoryCountsArtifact::from(evidence.category_counts),
            top_category: evidence.top_category.map(format_category),
            row_id_sample: evidence
                .selected_row_id_sample
                .iter()
                .map(|row_id| row_id.0)
                .collect(),
            selected_record_sample: evidence
                .selected_record_sample
                .iter()
                .map(SelectedPointSampleArtifact::from)
                .collect(),
        }
    }
}

impl From<&ScatterSelectionEvidenceV2> for ScatterSelectionEvidenceV2Artifact {
    fn from(evidence: &ScatterSelectionEvidenceV2) -> Self {
        Self {
            artifact_kind: SCATTER_SELECTION_EVIDENCE_V2_ARTIFACT_KIND,
            schema_version: SCATTER_SELECTION_EVIDENCE_V2_SCHEMA_VERSION,
            dataset_identity: DatasetIdentityArtifact::from(&evidence.dataset_identity),
            view: ScatterEvidenceViewArtifact::from(evidence.view),
            brush_range: BrushRangeArtifact {
                x: RangeArtifact::from(evidence.brush_x_range),
                y: RangeArtifact::from(evidence.brush_y_range),
            },
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            selected_extent: SelectedExtentArtifact {
                x: evidence.selected_x_range.map(RangeArtifact::from),
                y: evidence.selected_y_range.map(RangeArtifact::from),
            },
            point_kind_counts: PointKindCountsArtifact::from(evidence.point_kind_counts),
            top_point_kind: evidence.top_point_kind.map(format_scatter_point_kind),
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
struct BrushRangeArtifact {
    x: RangeArtifact,
    y: RangeArtifact,
}

#[derive(Debug, Serialize)]
struct ScatterEvidenceViewArtifact {
    x_range: RangeArtifact,
    y_range: RangeArtifact,
    grid_width: u32,
    grid_height: u32,
}

impl From<ScatterEvidenceView> for ScatterEvidenceViewArtifact {
    fn from(view: ScatterEvidenceView) -> Self {
        Self {
            x_range: RangeArtifact::from(view.x_range),
            y_range: RangeArtifact::from(view.y_range),
            grid_width: view.grid_width,
            grid_height: view.grid_height,
        }
    }
}

#[derive(Debug, Serialize)]
struct SelectedExtentArtifact {
    x: Option<RangeArtifact>,
    y: Option<RangeArtifact>,
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
struct CategoryCountsArtifact {
    cluster: usize,
    background: usize,
    outlier: usize,
}

#[derive(Debug, Serialize)]
struct PointKindCountsArtifact {
    cluster: usize,
    background: usize,
    outlier: usize,
    unclassified: usize,
}

impl From<ScatterSelectionKindCounts> for PointKindCountsArtifact {
    fn from(counts: ScatterSelectionKindCounts) -> Self {
        Self {
            cluster: counts.cluster,
            background: counts.background,
            outlier: counts.outlier,
            unclassified: counts.unclassified,
        }
    }
}

impl From<ScatterSelectionKindCounts> for CategoryCountsArtifact {
    fn from(counts: ScatterSelectionKindCounts) -> Self {
        Self {
            cluster: counts.cluster,
            background: counts.background,
            outlier: counts.outlier,
        }
    }
}

#[derive(Debug, Serialize)]
struct SelectedPointSampleArtifact {
    row_id: u64,
    x: f32,
    y: f32,
    category: String,
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

impl From<&SelectedPointSample> for SelectedPointSampleArtifact {
    fn from(sample: &SelectedPointSample) -> Self {
        Self {
            row_id: sample.row_id.0,
            x: sample.x,
            y: sample.y,
            category: format_optional_category(sample.category),
        }
    }
}

fn format_optional_range(range: Option<F32Range>) -> String {
    range
        .map(|range| format!("{:.6}..{:.6}", range.min, range.max))
        .unwrap_or_else(|| "none".to_string())
}

fn format_optional_category(category: Option<SyntheticPointCategory>) -> String {
    category
        .map(format_category)
        .unwrap_or_else(|| "none".to_string())
}

fn format_category(category: SyntheticPointCategory) -> String {
    format!("{category:?}")
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
        DatasetFieldRole::Value => "value".to_string(),
        DatasetFieldRole::Category => "category".to_string(),
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

fn format_scatter_point_kind(kind: ScatterPointKind) -> String {
    match kind {
        ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster) => "cluster".to_string(),
        ScatterPointKind::Synthetic(SyntheticPointCategory::Background) => "background".to_string(),
        ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier) => "outlier".to_string(),
        ScatterPointKind::Unclassified => "unclassified".to_string(),
    }
}
