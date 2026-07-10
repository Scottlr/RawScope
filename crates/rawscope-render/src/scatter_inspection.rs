//! Settled scatter-density inspection cache with bounded row evidence.

use std::{error::Error, fmt, sync::Arc};

use rawscope_core::{F32Range, RowId};
use rawscope_data::{FilterMask, FilterRevision, ScatterPointRecord};

use crate::{BrushScreenRect, BrushScreenSize, MaskAlignmentError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterInspectionConfig {
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_row_ids_per_bin: usize,
}

impl Default for ScatterInspectionConfig {
    fn default() -> Self {
        Self {
            grid_width: 256,
            grid_height: 256,
            max_row_ids_per_bin: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScatterInspectionBin {
    pub count: u32,
    pub row_ids: Arc<[RowId]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterInspectionGrid {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub filter_revision: FilterRevision,
    pub bins: Vec<ScatterInspectionBin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterInspectionHit {
    pub bin_x: u32,
    pub bin_y: u32,
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub count: u32,
    pub row_ids: Arc<[RowId]>,
}

impl ScatterInspectionHit {
    pub fn screen_rect(
        &self,
        grid_width: u32,
        grid_height: u32,
        screen_size: BrushScreenSize,
    ) -> Option<BrushScreenRect> {
        if grid_width == 0
            || grid_height == 0
            || self.bin_x >= grid_width
            || self.bin_y >= grid_height
            || screen_size.width <= 0.0
            || screen_size.height <= 0.0
        {
            return None;
        }
        let screen_row = grid_height - 1 - self.bin_y;
        Some(BrushScreenRect {
            min_x: self.bin_x as f32 / grid_width as f32 * screen_size.width,
            max_x: (self.bin_x + 1) as f32 / grid_width as f32 * screen_size.width,
            min_y: screen_row as f32 / grid_height as f32 * screen_size.height,
            max_y: (screen_row + 1) as f32 / grid_height as f32 * screen_size.height,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterInspectionError {
    EmptyGrid,
    GridTooLarge,
    MaskAlignment(MaskAlignmentError),
}

impl fmt::Display for ScatterInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrid => write!(formatter, "inspection grid dimensions must be positive"),
            Self::GridTooLarge => write!(formatter, "inspection grid dimensions are too large"),
            Self::MaskAlignment(error) => error.fmt(formatter),
        }
    }
}

impl Error for ScatterInspectionError {}

pub fn build_scatter_inspection_grid(
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    x_range: F32Range,
    y_range: F32Range,
    filter_revision: FilterRevision,
    config: ScatterInspectionConfig,
) -> Result<ScatterInspectionGrid, ScatterInspectionError> {
    if config.grid_width == 0 || config.grid_height == 0 {
        return Err(ScatterInspectionError::EmptyGrid);
    }
    MaskAlignmentError::require(points.len(), mask.len())
        .map_err(ScatterInspectionError::MaskAlignment)?;
    let bin_count = usize::try_from(config.grid_width)
        .ok()
        .and_then(|width| {
            usize::try_from(config.grid_height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(ScatterInspectionError::GridTooLarge)?;
    let mut bins = (0..bin_count)
        .map(|_| MutableInspectionBin::new(config.max_row_ids_per_bin))
        .collect::<Vec<_>>();
    for (point, included) in points.iter().zip(mask.as_gpu_u32_slice()) {
        if *included == 0 {
            continue;
        }
        let (Some(bin_x), Some(bin_y)) = (
            bin_f32(point.x, x_range, config.grid_width),
            bin_f32(point.y, y_range, config.grid_height),
        ) else {
            continue;
        };
        let bin_index = bin_y as usize * config.grid_width as usize + bin_x as usize;
        bins[bin_index].record(point.row_id, config.max_row_ids_per_bin);
    }
    Ok(ScatterInspectionGrid {
        x_range,
        y_range,
        grid_width: config.grid_width,
        grid_height: config.grid_height,
        filter_revision,
        bins: bins.into_iter().map(ScatterInspectionBin::from).collect(),
    })
}

impl ScatterInspectionGrid {
    pub fn inspect_fraction(
        &self,
        x_fraction: f32,
        y_fraction: f32,
    ) -> Option<ScatterInspectionHit> {
        if !(0.0..=1.0).contains(&x_fraction) || !(0.0..=1.0).contains(&y_fraction) {
            return None;
        }
        let data_x = self.x_range.min + x_fraction * self.x_range.span();
        let data_y = self.y_range.max - y_fraction * self.y_range.span();
        let bin_x = bin_f32(data_x, self.x_range, self.grid_width)?;
        let bin_y = bin_f32(data_y, self.y_range, self.grid_height)?;
        let bin = self
            .bins
            .get(bin_y as usize * self.grid_width as usize + bin_x as usize)?;
        Some(ScatterInspectionHit {
            bin_x,
            bin_y,
            x_range: bin_range(self.x_range, bin_x, self.grid_width),
            y_range: bin_range(self.y_range, bin_y, self.grid_height),
            count: bin.count,
            row_ids: Arc::clone(&bin.row_ids),
        })
    }
}

struct MutableInspectionBin {
    count: u32,
    row_ids: Vec<RowId>,
}

impl MutableInspectionBin {
    fn new(sample_limit: usize) -> Self {
        Self {
            count: 0,
            row_ids: Vec::with_capacity(sample_limit),
        }
    }

    fn record(&mut self, row_id: RowId, sample_limit: usize) {
        self.count = self.count.saturating_add(1);
        let insertion = self
            .row_ids
            .binary_search(&row_id)
            .unwrap_or_else(|index| index);
        if insertion < sample_limit {
            self.row_ids.insert(insertion, row_id);
            self.row_ids.truncate(sample_limit);
        }
    }
}

impl From<MutableInspectionBin> for ScatterInspectionBin {
    fn from(bin: MutableInspectionBin) -> Self {
        Self {
            count: bin.count,
            row_ids: bin.row_ids.into(),
        }
    }
}

fn bin_f32(value: f32, range: F32Range, bin_count: u32) -> Option<u32> {
    if !range.contains(value) {
        return None;
    }
    if value == range.max {
        return Some(bin_count - 1);
    }
    let normalized = (value - range.min) / range.span();
    Some(((normalized * bin_count as f32).floor() as u32).min(bin_count - 1))
}

fn bin_range(range: F32Range, bin: u32, bin_count: u32) -> F32Range {
    let bin_span = range.span() / bin_count as f32;
    let min = range.min + bin as f32 * bin_span;
    let max = if bin + 1 == bin_count {
        range.max
    } else {
        min + bin_span
    };
    F32Range::new(min, max)
}

#[cfg(test)]
mod tests {
    use rawscope_data::{
        build_visual_field_catalog, evaluate_filters, DatasetFilter, FilterSet, LoadedColumnKind,
        LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
        VisualFieldCatalogConfig,
    };

    use super::*;

    #[test]
    fn inspection_grid_matches_density_bin_edges() {
        let points = points(&[(0, 0.0, 0.0), (1, 10.0, 10.0), (2, 5.0, 5.0)]);
        let evaluation = evaluation(&["keep", "keep", "keep"], None);
        let grid = build(&points, &evaluation, 10, 10, 16);

        assert_eq!(grid.bins[0].count, 1);
        assert_eq!(grid.bins[99].count, 1);
        assert_eq!(grid.bins[55].count, 1);
    }

    #[test]
    fn inspection_grid_counts_all_rows_but_bounds_samples() {
        let points = points(&[(9, 1.0, 1.0), (2, 1.0, 1.0), (7, 1.0, 1.0)]);
        let evaluation = evaluation(&["keep", "keep", "keep"], None);
        let grid = build(&points, &evaluation, 1, 1, 2);

        assert_eq!(grid.bins[0].count, 3);
        assert_eq!(&*grid.bins[0].row_ids, &[RowId(2), RowId(7)]);
    }

    #[test]
    fn inspection_grid_respects_filter_mask() {
        let points = points(&[(0, 1.0, 1.0), (1, 1.0, 1.0), (2, 1.0, 1.0)]);
        let evaluation = evaluation(&["keep", "drop", "keep"], Some("keep"));
        let grid = build(&points, &evaluation, 1, 1, 16);

        assert_eq!(grid.bins[0].count, 2);
        assert_eq!(&*grid.bins[0].row_ids, &[RowId(0), RowId(2)]);
    }

    #[test]
    fn hover_lookup_maps_screen_y_to_data_y() {
        let points = points(&[(0, 1.0, 9.0)]);
        let evaluation = evaluation(&["keep"], None);
        let grid = build(&points, &evaluation, 10, 10, 16);
        let hit = grid.inspect_fraction(0.1, 0.1).unwrap();

        assert_eq!((hit.bin_x, hit.bin_y), (1, 9));
        assert_eq!(hit.count, 1);
        assert_eq!(hit.y_range, F32Range::new(9.0, 10.0));
    }

    fn build(
        points: &[ScatterPointRecord],
        evaluation: &rawscope_data::FilterEvaluation,
        width: u32,
        height: u32,
        sample_limit: usize,
    ) -> ScatterInspectionGrid {
        build_scatter_inspection_grid(
            points,
            &evaluation.mask,
            F32Range::new(0.0, 10.0),
            F32Range::new(0.0, 10.0),
            evaluation.revision,
            ScatterInspectionConfig {
                grid_width: width,
                grid_height: height,
                max_row_ids_per_bin: sample_limit,
            },
        )
        .unwrap()
    }

    fn evaluation(values: &[&str], included: Option<&str>) -> rawscope_data::FilterEvaluation {
        let source = LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "cohort".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: values
                .iter()
                .enumerate()
                .map(|(index, value)| LoadedSourceRow {
                    row_id: RowId(index as u64),
                    values: vec![(*value).into()],
                })
                .collect(),
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let mut filters = FilterSet::default();
        if let Some(value) = included {
            filters.replace_for_column(DatasetFilter::Categories {
                column_name: "cohort".into(),
                included_values: vec![value.into()],
                include_missing: false,
            });
        }
        evaluate_filters(&source, &catalog, &filters).unwrap()
    }

    fn points(values: &[(u64, f32, f32)]) -> Vec<ScatterPointRecord> {
        values
            .iter()
            .map(|(row_id, x, y)| ScatterPointRecord {
                row_id: RowId(*row_id),
                x: *x,
                y: *y,
                kind: ScatterPointKind::Unclassified,
            })
            .collect()
    }
}
