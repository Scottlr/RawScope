//! CPU-side row evidence for finalized scatter brush selections.

use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    DatasetIdentity, LoadedSourceRow, LoadedSourceTable, ScatterPointKind, ScatterPointRecord,
    SyntheticDatasetMetadata, SyntheticPointCategory,
};

use crate::evidence_sample::{insert_lowest_row_id_sample, RowIdSample};

const DEFAULT_MAX_SAMPLE_SIZE: usize = 10;

/// Data-space geometry used to build scatter evidence without render coupling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterSelectionGeometry {
    pub x_range: F32Range,
    pub y_range: F32Range,
}

/// Counts selected rows by synthetic point kind.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScatterSelectionKindCounts {
    pub cluster: usize,
    pub background: usize,
    pub outlier: usize,
    pub unclassified: usize,
}

impl ScatterSelectionKindCounts {
    pub fn top_category(self) -> Option<SyntheticPointCategory> {
        [
            (SyntheticPointCategory::Cluster, self.cluster),
            (SyntheticPointCategory::Background, self.background),
            (SyntheticPointCategory::Outlier, self.outlier),
        ]
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(category, count)| (count > 0).then_some(category))
    }

    pub fn top_point_kind(self) -> Option<ScatterPointKind> {
        [
            (
                ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
                self.cluster,
            ),
            (
                ScatterPointKind::Synthetic(SyntheticPointCategory::Background),
                self.background,
            ),
            (
                ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier),
                self.outlier,
            ),
            (ScatterPointKind::Unclassified, self.unclassified),
        ]
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(kind, count)| (count > 0).then_some(kind))
    }
}

/// Configuration for deterministic scatter selection evidence building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionEvidenceConfig {
    pub max_sample_size: usize,
}

impl Default for SelectionEvidenceConfig {
    fn default() -> Self {
        Self {
            max_sample_size: DEFAULT_MAX_SAMPLE_SIZE,
        }
    }
}

/// Small sampled scatter point record included in selection evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedPointSample {
    pub row_id: RowId,
    pub x: f32,
    pub y: f32,
    pub category: Option<SyntheticPointCategory>,
}

impl From<&ScatterPointRecord> for SelectedPointSample {
    fn from(point: &ScatterPointRecord) -> Self {
        Self {
            row_id: point.row_id,
            x: point.x,
            y: point.y,
            category: point.kind.synthetic_category(),
        }
    }
}

impl RowIdSample for SelectedPointSample {
    fn row_id(&self) -> RowId {
        self.row_id
    }
}

/// View configuration included in scatter evidence v2 artifacts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterEvidenceView {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
}

/// Small sampled scatter point record included in selection evidence v2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedPointSampleV2 {
    pub row_id: RowId,
    pub x: f32,
    pub y: f32,
    pub kind: ScatterPointKind,
}

impl From<&SelectedPointSample> for SelectedPointSampleV2 {
    fn from(sample: &SelectedPointSample) -> Self {
        let kind = sample
            .category
            .map(ScatterPointKind::Synthetic)
            .unwrap_or(ScatterPointKind::Unclassified);
        Self {
            row_id: sample.row_id,
            x: sample.x,
            y: sample.y,
            kind,
        }
    }
}

/// Deterministic retained source-row sample included in evidence v2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedSourceRowSample {
    pub row_id: RowId,
    pub values: Vec<String>,
}

impl From<&LoadedSourceRow> for SelectedSourceRowSample {
    fn from(row: &LoadedSourceRow) -> Self {
        Self {
            row_id: row.row_id,
            values: row.values.clone(),
        }
    }
}

/// Deterministic CPU-side evidence for a finalized scatter brush selection.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionEvidence {
    pub dataset_metadata: SyntheticDatasetMetadata,
    pub point_preset_row_count: usize,
    pub selected_row_count: usize,
    pub selected_percentage: f32,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_record_sample: Vec<SelectedPointSample>,
    pub category_counts: ScatterSelectionKindCounts,
    pub top_category: Option<SyntheticPointCategory>,
    pub selected_x_range: Option<F32Range>,
    pub selected_y_range: Option<F32Range>,
    pub brush_x_range: F32Range,
    pub brush_y_range: F32Range,
}

/// Source-aware CPU-side scatter selection evidence for v2 artifacts.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionEvidenceV2 {
    pub dataset_identity: DatasetIdentity,
    pub view: ScatterEvidenceView,
    pub selected_row_count: usize,
    pub selected_percentage: f32,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_record_sample: Vec<SelectedPointSampleV2>,
    pub selected_source_column_names: Vec<String>,
    pub selected_source_row_sample: Vec<SelectedSourceRowSample>,
    pub point_kind_counts: ScatterSelectionKindCounts,
    pub top_point_kind: Option<ScatterPointKind>,
    pub selected_x_range: Option<F32Range>,
    pub selected_y_range: Option<F32Range>,
    pub brush_x_range: F32Range,
    pub brush_y_range: F32Range,
}

impl ScatterSelectionEvidenceV2 {
    /// Builds source-aware evidence from cached v1 evidence plus current dataset context.
    pub fn from_v1(
        evidence: &ScatterSelectionEvidence,
        dataset_identity: DatasetIdentity,
        view: ScatterEvidenceView,
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
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            selected_row_id_sample: evidence.selected_row_id_sample.clone(),
            selected_record_sample: evidence
                .selected_record_sample
                .iter()
                .map(SelectedPointSampleV2::from)
                .collect(),
            selected_source_column_names,
            selected_source_row_sample,
            point_kind_counts: evidence.category_counts,
            top_point_kind: evidence.category_counts.top_point_kind(),
            selected_x_range: evidence.selected_x_range,
            selected_y_range: evidence.selected_y_range,
            brush_x_range: evidence.brush_x_range,
            brush_y_range: evidence.brush_y_range,
        }
    }
}

impl ScatterSelectionEvidence {
    /// Builds deterministic selected-row evidence from scatter point records.
    ///
    /// Sampling uses the lowest selected row ids so the same selection and dataset always produce
    /// the same evidence without random state.
    pub fn from_points(
        points: &[ScatterPointRecord],
        geometry: ScatterSelectionGeometry,
        dataset_metadata: SyntheticDatasetMetadata,
        point_preset_row_count: usize,
        config: SelectionEvidenceConfig,
    ) -> Self {
        let mut selected_row_count = 0;
        let mut selected_min_x = f32::INFINITY;
        let mut selected_max_x = f32::NEG_INFINITY;
        let mut selected_min_y = f32::INFINITY;
        let mut selected_max_y = f32::NEG_INFINITY;
        let mut category_counts = ScatterSelectionKindCounts::default();
        let mut selected_record_sample = Vec::new();

        for point in points {
            let point_is_selected =
                geometry.x_range.contains(point.x) && geometry.y_range.contains(point.y);
            if !point_is_selected {
                continue;
            }

            selected_row_count += 1;
            selected_min_x = selected_min_x.min(point.x);
            selected_max_x = selected_max_x.max(point.x);
            selected_min_y = selected_min_y.min(point.y);
            selected_max_y = selected_max_y.max(point.y);
            add_category_count(&mut category_counts, point.kind);
            insert_lowest_row_id_sample(
                &mut selected_record_sample,
                SelectedPointSample::from(point),
                config.max_sample_size,
            );
        }

        let selected_percentage = if points.is_empty() {
            0.0
        } else {
            selected_row_count as f32 / points.len() as f32 * 100.0
        };
        let selected_row_id_sample = selected_record_sample
            .iter()
            .map(|sample| sample.row_id)
            .collect();

        Self {
            dataset_metadata,
            point_preset_row_count,
            selected_row_count,
            selected_percentage,
            selected_row_id_sample,
            selected_record_sample,
            category_counts,
            top_category: category_counts.top_category(),
            selected_x_range: selected_range(selected_row_count, selected_min_x, selected_max_x),
            selected_y_range: selected_range(selected_row_count, selected_min_y, selected_max_y),
            brush_x_range: geometry.x_range,
            brush_y_range: geometry.y_range,
        }
    }
}

fn add_category_count(counts: &mut ScatterSelectionKindCounts, kind: ScatterPointKind) {
    match kind {
        ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster) => counts.cluster += 1,
        ScatterPointKind::Synthetic(SyntheticPointCategory::Background) => counts.background += 1,
        ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier) => counts.outlier += 1,
        ScatterPointKind::Unclassified => counts.unclassified += 1,
    }
}

fn selected_range(selected_row_count: usize, min: f32, max: f32) -> Option<F32Range> {
    if selected_row_count == 0 {
        return None;
    }

    Some(F32Range::from_bounds_expanded(min, max))
}
