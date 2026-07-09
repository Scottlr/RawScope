//! JSON and Markdown artifacts for scatter selection evidence.

use rawscope_core::F32Range;
use rawscope_data::{SyntheticDatasetMetadata, SyntheticPointCategory};
use serde::Serialize;

use crate::{ScatterSelectionEvidence, SelectedCategoryCounts, SelectedPointSample};

/// Artifact kind used by scatter selection evidence JSON and export manifests.
pub const SCATTER_SELECTION_EVIDENCE_ARTIFACT_KIND: &str = "scatter-selection-evidence";

/// Schema version for scatter selection evidence artifacts.
pub const SCATTER_SELECTION_EVIDENCE_SCHEMA_VERSION: u32 = 1;

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
struct BrushRangeArtifact {
    x: RangeArtifact,
    y: RangeArtifact,
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

impl From<SelectedCategoryCounts> for CategoryCountsArtifact {
    fn from(counts: SelectedCategoryCounts) -> Self {
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
