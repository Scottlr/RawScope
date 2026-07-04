//! CPU-side row evidence for finalized scatter brush selections.

use rawscope_core::{F32Range, RowId};
use rawscope_data::{SyntheticDatasetMetadata, SyntheticPointCategory, SyntheticPointRecord};

use crate::evidence_sample::{insert_lowest_row_id_sample, RowIdSample};
use crate::{ScatterBrushSelection, SelectedCategoryCounts};

const DEFAULT_MAX_SAMPLE_SIZE: usize = 10;

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

/// Small sampled synthetic point record included in selection evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedPointSample {
    pub row_id: RowId,
    pub x: f32,
    pub y: f32,
    pub category: SyntheticPointCategory,
}

impl From<&SyntheticPointRecord> for SelectedPointSample {
    fn from(point: &SyntheticPointRecord) -> Self {
        Self {
            row_id: point.row_id,
            x: point.x,
            y: point.y,
            category: point.category,
        }
    }
}

impl RowIdSample for SelectedPointSample {
    fn row_id(&self) -> RowId {
        self.row_id
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
    pub category_counts: SelectedCategoryCounts,
    pub top_category: Option<SyntheticPointCategory>,
    pub selected_x_range: Option<F32Range>,
    pub selected_y_range: Option<F32Range>,
    pub brush_x_range: F32Range,
    pub brush_y_range: F32Range,
}

impl ScatterSelectionEvidence {
    /// Builds deterministic selected-row evidence from synthetic point records.
    ///
    /// Sampling uses the lowest selected row ids so the same selection and dataset always produce
    /// the same evidence without random state.
    pub fn from_points(
        points: &[SyntheticPointRecord],
        selection: ScatterBrushSelection,
        dataset_metadata: SyntheticDatasetMetadata,
        point_preset_row_count: usize,
        config: SelectionEvidenceConfig,
    ) -> Self {
        let mut selected_row_count = 0;
        let mut selected_min_x = f32::INFINITY;
        let mut selected_max_x = f32::NEG_INFINITY;
        let mut selected_min_y = f32::INFINITY;
        let mut selected_max_y = f32::NEG_INFINITY;
        let mut category_counts = SelectedCategoryCounts::default();
        let mut selected_record_sample = Vec::new();

        for point in points {
            let point_is_selected = selection.contains_point(point);
            if !point_is_selected {
                continue;
            }

            selected_row_count += 1;
            selected_min_x = selected_min_x.min(point.x);
            selected_max_x = selected_max_x.max(point.x);
            selected_min_y = selected_min_y.min(point.y);
            selected_max_y = selected_max_y.max(point.y);
            add_category_count(&mut category_counts, point.category);
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
            brush_x_range: selection.x_range,
            brush_y_range: selection.y_range,
        }
    }
}

fn add_category_count(counts: &mut SelectedCategoryCounts, category: SyntheticPointCategory) {
    match category {
        SyntheticPointCategory::Cluster => counts.cluster += 1,
        SyntheticPointCategory::Background => counts.background += 1,
        SyntheticPointCategory::Outlier => counts.outlier += 1,
    }
}

fn selected_range(selected_row_count: usize, min: f32, max: f32) -> Option<F32Range> {
    if selected_row_count == 0 {
        return None;
    }

    Some(F32Range::from_bounds_expanded(min, max))
}
