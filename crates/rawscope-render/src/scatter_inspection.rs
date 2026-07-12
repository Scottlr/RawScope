//! Settled scatter-density inspection cache with bounded row evidence.

use std::{collections::BTreeMap, error::Error, fmt, num::NonZeroU32, sync::Arc};

use rawscope_analysis::density::{bin_f32, BinIndex, BinPlacement};
use rawscope_core::{F32Range, RowId};
use rawscope_data::{FilterMask, FilterRevision, ScatterPointRecord};

use crate::{BrushScreenRect, BrushScreenSize, MaskAlignmentError};

pub const INSPECTION_NEIGHBORHOOD_RADIUS_BINS: u32 = 1;

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
    pub distribution: ScatterInspectionDistribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterInspectionDistribution {
    pub included_row_count: usize,
    pub occupied_bin_count: u32,
    count_histogram: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterInspectionSummary {
    pub hit: ScatterInspectionHit,
    pub active_share: f64,
    pub occupied_density_percentile: Option<f64>,
    pub neighborhood_radius_bins: u32,
    pub neighborhood_count: u64,
    pub neighborhood_share: f64,
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
        .map(|_| MutableInspectionBin::new())
        .collect::<Vec<_>>();
    for (point, included) in points.iter().zip(mask.as_gpu_u32_slice()) {
        if *included == 0 {
            continue;
        }
        let (Some(bin_x), Some(bin_y)) = (
            in_domain_bin(bin_f32(point.x, x_range, non_zero_bins(config.grid_width))),
            in_domain_bin(bin_f32(point.y, y_range, non_zero_bins(config.grid_height))),
        ) else {
            continue;
        };
        let bin_index = bin_y as usize * config.grid_width as usize + bin_x as usize;
        bins[bin_index].record(point.row_id, config.max_row_ids_per_bin);
    }
    let bins = bins
        .into_iter()
        .map(ScatterInspectionBin::from)
        .collect::<Vec<_>>();
    let distribution = inspection_distribution(&bins, mask.included_count());
    Ok(ScatterInspectionGrid {
        x_range,
        y_range,
        grid_width: config.grid_width,
        grid_height: config.grid_height,
        filter_revision,
        bins,
        distribution,
    })
}

impl ScatterInspectionGrid {
    pub fn inspect_bin(&self, bin_x: u32, bin_y: u32) -> Option<ScatterInspectionHit> {
        if bin_x >= self.grid_width || bin_y >= self.grid_height {
            return None;
        }
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
        let bin_x = in_domain_bin(bin_f32(
            data_x,
            self.x_range,
            non_zero_bins(self.grid_width),
        ))?;
        let bin_y = in_domain_bin(bin_f32(
            data_y,
            self.y_range,
            non_zero_bins(self.grid_height),
        ))?;
        self.inspect_bin(bin_x, bin_y)
    }

    pub fn summarize_hit(&self, hit: ScatterInspectionHit) -> ScatterInspectionSummary {
        let active_share = if self.distribution.included_row_count == 0 {
            0.0
        } else {
            f64::from(hit.count) / self.distribution.included_row_count as f64
        };
        let neighborhood_count = self.neighborhood_count(hit.bin_x, hit.bin_y);
        let neighborhood_share = if self.distribution.included_row_count == 0 {
            0.0
        } else {
            neighborhood_count as f64 / self.distribution.included_row_count as f64
        };
        ScatterInspectionSummary {
            occupied_density_percentile: self.density_percentile(hit.count),
            hit,
            active_share,
            neighborhood_radius_bins: INSPECTION_NEIGHBORHOOD_RADIUS_BINS,
            neighborhood_count,
            neighborhood_share,
        }
    }

    fn density_percentile(&self, count: u32) -> Option<f64> {
        if count == 0 || self.distribution.occupied_bin_count == 0 {
            return None;
        }
        let at_or_below_count = self
            .distribution
            .count_histogram
            .iter()
            .take_while(|(bin_count, _)| *bin_count <= count)
            .map(|(_, bin_frequency)| u64::from(*bin_frequency))
            .sum::<u64>();
        Some(at_or_below_count as f64 / self.distribution.occupied_bin_count as f64)
    }

    fn neighborhood_count(&self, bin_x: u32, bin_y: u32) -> u64 {
        let first_x = bin_x.saturating_sub(INSPECTION_NEIGHBORHOOD_RADIUS_BINS);
        let last_x = bin_x
            .saturating_add(INSPECTION_NEIGHBORHOOD_RADIUS_BINS)
            .min(self.grid_width - 1);
        let first_y = bin_y.saturating_sub(INSPECTION_NEIGHBORHOOD_RADIUS_BINS);
        let last_y = bin_y
            .saturating_add(INSPECTION_NEIGHBORHOOD_RADIUS_BINS)
            .min(self.grid_height - 1);
        let mut count = 0_u64;
        for neighbor_y in first_y..=last_y {
            for neighbor_x in first_x..=last_x {
                let index = neighbor_y as usize * self.grid_width as usize + neighbor_x as usize;
                count += u64::from(self.bins[index].count);
            }
        }
        count
    }
}

fn inspection_distribution(
    bins: &[ScatterInspectionBin],
    included_row_count: usize,
) -> ScatterInspectionDistribution {
    let mut histogram = BTreeMap::<u32, u32>::new();
    for bin in bins {
        if bin.count > 0 {
            *histogram.entry(bin.count).or_default() += 1;
        }
    }
    ScatterInspectionDistribution {
        included_row_count,
        occupied_bin_count: histogram.values().copied().sum(),
        count_histogram: histogram.into_iter().collect(),
    }
}

struct MutableInspectionBin {
    count: u32,
    row_ids: Vec<RowId>,
}

impl MutableInspectionBin {
    fn new() -> Self {
        Self {
            count: 0,
            row_ids: Vec::new(),
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

fn non_zero_bins(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("inspection grid dimensions are validated")
}

fn in_domain_bin(
    result: Result<BinPlacement, rawscope_analysis::density::BinningError>,
) -> Option<u32> {
    match result.ok()? {
        BinPlacement::InDomain(BinIndex(index)) => Some(index),
        BinPlacement::BeforeDomain | BinPlacement::AfterDomain => None,
    }
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

    #[test]
    fn inspection_summary_reports_exact_active_share() {
        let points = points(&[(0, 1.0, 1.0), (1, 1.0, 1.0), (2, 9.0, 9.0)]);
        let evaluation = evaluation(&["keep", "keep", "keep"], None);
        let grid = build(&points, &evaluation, 2, 2, 16);

        let summary = grid.summarize_hit(grid.inspect_bin(0, 0).unwrap());

        assert!((summary.active_share - 2.0 / 3.0).abs() < f64::EPSILON * 4.0);
        assert_eq!(summary.hit.count, 2);
    }

    #[test]
    fn density_percentile_is_tie_inclusive_over_occupied_cells() {
        let points = points(&[(0, 1.0, 1.0), (1, 1.0, 1.0), (2, 9.0, 9.0), (3, 1.0, 9.0)]);
        let evaluation = evaluation(&["keep", "keep", "keep", "keep"], None);
        let grid = build(&points, &evaluation, 2, 2, 16);

        let summary = grid.summarize_hit(grid.inspect_bin(1, 1).unwrap());

        assert_eq!(grid.distribution.occupied_bin_count, 3);
        assert_eq!(summary.occupied_density_percentile, Some(2.0 / 3.0));
    }

    #[test]
    fn empty_cell_has_no_density_percentile() {
        let points = points(&[(0, 1.0, 1.0)]);
        let evaluation = evaluation(&["keep"], None);
        let grid = build(&points, &evaluation, 2, 2, 16);

        let summary = grid.summarize_hit(grid.inspect_bin(1, 0).unwrap());

        assert_eq!(summary.hit.count, 0);
        assert_eq!(summary.occupied_density_percentile, None);
    }

    #[test]
    fn neighborhood_clips_at_grid_edges_without_double_counting() {
        let points = points(&[(0, 1.0, 1.0), (1, 4.0, 1.0), (2, 1.0, 4.0), (3, 4.0, 4.0)]);
        let evaluation = evaluation(&["keep", "keep", "keep", "keep"], None);
        let grid = build(&points, &evaluation, 3, 3, 16);

        let summary = grid.summarize_hit(grid.inspect_bin(0, 0).unwrap());

        assert_eq!(summary.neighborhood_radius_bins, 1);
        assert_eq!(summary.neighborhood_count, 4);
        assert_eq!(summary.neighborhood_share, 1.0);
    }

    #[test]
    fn summary_reuses_bounded_row_sample() {
        let points = points(&[(9, 1.0, 1.0), (2, 1.0, 1.0), (7, 1.0, 1.0)]);
        let evaluation = evaluation(&["keep", "keep", "keep"], None);
        let grid = build(&points, &evaluation, 1, 1, 1);

        let summary = grid.summarize_hit(grid.inspect_bin(0, 0).unwrap());

        assert_eq!(summary.hit.count, 3);
        assert_eq!(&*summary.hit.row_ids, &[RowId(2)]);
    }

    #[test]
    fn empty_inspection_bins_do_not_reserve_sample_capacity() {
        let bin = MutableInspectionBin::new();

        assert_eq!(bin.row_ids.capacity(), 0);
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
