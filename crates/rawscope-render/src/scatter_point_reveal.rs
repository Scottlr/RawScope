//! Deterministic point-reveal level-of-detail policy and sampling.

use std::{error::Error, fmt};

use rawscope_core::{F32Range, RowId};
use rawscope_data::{FilterMask, ScatterPointRecord};

use crate::MaskAlignmentError;

const DEFAULT_MAX_RENDERED_POINTS: usize = 20_000;
const DEFAULT_FULLY_VISIBLE_ROWS_PER_PIXEL: f32 = 0.02;
const DEFAULT_HIDDEN_ROWS_PER_PIXEL: f32 = 0.12;
const DEFAULT_POINT_RADIUS_PX: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointRevealMode {
    Off,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointRevealConfig {
    pub mode: PointRevealMode,
    pub max_rendered_points: usize,
    pub fully_visible_rows_per_pixel: f32,
    pub hidden_rows_per_pixel: f32,
    pub radius_px: f32,
}

impl Default for PointRevealConfig {
    fn default() -> Self {
        Self {
            mode: PointRevealMode::Auto,
            max_rendered_points: DEFAULT_MAX_RENDERED_POINTS,
            fully_visible_rows_per_pixel: DEFAULT_FULLY_VISIBLE_ROWS_PER_PIXEL,
            hidden_rows_per_pixel: DEFAULT_HIDDEN_ROWS_PER_PIXEL,
            radius_px: DEFAULT_POINT_RADIUS_PX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointRevealSelection {
    pub point_indices: Vec<u32>,
    pub eligible_count: usize,
    pub blend: f32,
    pub sampled: bool,
}

impl PointRevealSelection {
    pub fn stats(&self) -> PointRevealStats {
        PointRevealStats {
            eligible_count: self.eligible_count,
            rendered_count: self.point_indices.len(),
            sampled: self.sampled,
            blend: self.blend,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointRevealStats {
    pub eligible_count: usize,
    pub rendered_count: usize,
    pub sampled: bool,
    pub blend: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointRevealError {
    MaskAlignment(MaskAlignmentError),
    EmptyPlot,
    InvalidThresholds,
    PointCountTooLarge,
}

impl fmt::Display for PointRevealError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaskAlignment(error) => error.fmt(formatter),
            Self::EmptyPlot => write!(formatter, "point reveal plot dimensions must be positive"),
            Self::InvalidThresholds => write!(
                formatter,
                "point reveal thresholds must be finite, non-negative, and increasing"
            ),
            Self::PointCountTooLarge => write!(formatter, "point count exceeds u32::MAX"),
        }
    }
}

impl Error for PointRevealError {}

pub fn select_points_for_reveal(
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    x_range: F32Range,
    y_range: F32Range,
    plot_width_px: u32,
    plot_height_px: u32,
    config: PointRevealConfig,
) -> Result<PointRevealSelection, PointRevealError> {
    MaskAlignmentError::require(points.len(), mask.len())
        .map_err(PointRevealError::MaskAlignment)?;
    if plot_width_px == 0 || plot_height_px == 0 {
        return Err(PointRevealError::EmptyPlot);
    }
    let thresholds_are_valid = config.fully_visible_rows_per_pixel.is_finite()
        && config.hidden_rows_per_pixel.is_finite()
        && config.fully_visible_rows_per_pixel >= 0.0
        && config.hidden_rows_per_pixel > config.fully_visible_rows_per_pixel;
    if !thresholds_are_valid {
        return Err(PointRevealError::InvalidThresholds);
    }
    if points.len() > u32::MAX as usize {
        return Err(PointRevealError::PointCountTooLarge);
    }
    let eligible_count = points
        .iter()
        .zip(mask.as_gpu_u32_slice())
        .filter(|(point, included)| point_is_eligible(point, included, x_range, y_range))
        .count();
    if config.mode == PointRevealMode::Off {
        return Ok(PointRevealSelection {
            point_indices: Vec::new(),
            eligible_count,
            blend: 0.0,
            sampled: false,
        });
    }
    let plot_pixels = u64::from(plot_width_px) * u64::from(plot_height_px);
    let rows_per_pixel = eligible_count as f32 / plot_pixels as f32;
    let blend = reveal_blend(rows_per_pixel, config);
    if blend == 0.0 || config.max_rendered_points == 0 {
        return Ok(PointRevealSelection {
            point_indices: Vec::new(),
            eligible_count,
            blend,
            sampled: false,
        });
    }

    let mut eligible = points
        .iter()
        .zip(mask.as_gpu_u32_slice())
        .enumerate()
        .filter(|(_, (point, included))| point_is_eligible(point, included, x_range, y_range))
        .map(|(index, (point, _))| {
            (
                point_reveal_priority(point.row_id),
                point.row_id,
                index as u32,
            )
        })
        .collect::<Vec<_>>();
    if eligible.len() > config.max_rendered_points {
        eligible.select_nth_unstable_by_key(config.max_rendered_points, |(priority, row_id, _)| {
            (*priority, *row_id)
        });
        eligible.truncate(config.max_rendered_points);
    }
    eligible.sort_unstable_by_key(|(_, row_id, _)| *row_id);
    Ok(PointRevealSelection {
        point_indices: eligible.into_iter().map(|(_, _, index)| index).collect(),
        eligible_count,
        blend,
        sampled: eligible_count > config.max_rendered_points,
    })
}

fn point_is_eligible(
    point: &ScatterPointRecord,
    included: &u32,
    x_range: F32Range,
    y_range: F32Range,
) -> bool {
    *included == 1 && x_range.contains(point.x) && y_range.contains(point.y)
}

pub fn project_point_to_plot_fraction(
    point: &ScatterPointRecord,
    x_range: F32Range,
    y_range: F32Range,
) -> Option<(f32, f32)> {
    if !x_range.contains(point.x) || !y_range.contains(point.y) {
        return None;
    }
    Some((
        (point.x - x_range.min) / x_range.span(),
        (y_range.max - point.y) / y_range.span(),
    ))
}

fn reveal_blend(rows_per_pixel: f32, config: PointRevealConfig) -> f32 {
    if rows_per_pixel <= config.fully_visible_rows_per_pixel {
        return 1.0;
    }
    if rows_per_pixel >= config.hidden_rows_per_pixel {
        return 0.0;
    }
    1.0 - (rows_per_pixel - config.fully_visible_rows_per_pixel)
        / (config.hidden_rows_per_pixel - config.fully_visible_rows_per_pixel)
}

fn point_reveal_priority(row_id: RowId) -> u64 {
    let mut value = row_id.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
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
    fn point_reveal_blend_follows_rows_per_pixel() {
        let points = points(50);
        let evaluation = evaluation(50, None);
        let mut config = PointRevealConfig::default();
        config.fully_visible_rows_per_pixel = 0.03;
        config.hidden_rows_per_pixel = 0.25;

        let sparse = select(&points, &evaluation.mask, 100, 100, config);
        let transitional = select(&points, &evaluation.mask, 20, 20, config);
        let dense = select(&points, &evaluation.mask, 10, 10, config);

        assert_eq!(sparse.blend, 1.0);
        assert!(transitional.blend > 0.0 && transitional.blend < 1.0);
        assert_eq!(dense.blend, 0.0);
    }

    #[test]
    fn lichess_cohort_thresholds_cover_hidden_sampled_and_full_reveal() {
        let config = PointRevealConfig::default();
        let plot_pixels = (860 * 580) as f32;
        let full = reveal_blend(200_000.0 / plot_pixels, config);
        let black_wins = reveal_blend(94_499.0 / plot_pixels, config);
        let transitional = reveal_blend(50_000.0 / plot_pixels, config);
        let draws = reveal_blend(5_145.0 / plot_pixels, config);

        assert_eq!(full, 0.0);
        assert_eq!(black_wins, 0.0);
        assert!(transitional > 0.0 && transitional < 1.0);
        assert_eq!(draws, 1.0);
    }

    #[test]
    fn dense_auto_reveal_counts_rows_without_selecting_markers() {
        let points = points(100);
        let evaluation = evaluation(100, None);
        let selection = select(
            &points,
            &evaluation.mask,
            10,
            10,
            PointRevealConfig::default(),
        );

        assert_eq!(selection.eligible_count, 100);
        assert!(selection.point_indices.is_empty());
        assert_eq!(selection.blend, 0.0);
        assert!(!selection.sampled);
    }

    #[test]
    fn point_reveal_never_exceeds_budget() {
        let points = points(100);
        let evaluation = evaluation(100, None);
        let selection = select(
            &points,
            &evaluation.mask,
            1_000,
            1_000,
            PointRevealConfig {
                max_rendered_points: 7,
                ..Default::default()
            },
        );

        assert_eq!(selection.point_indices.len(), 7);
        assert!(selection.sampled);
    }

    #[test]
    fn point_reveal_sample_is_stable_by_row_id() {
        let points = points(100);
        let evaluation = evaluation(100, None);
        let config = PointRevealConfig {
            max_rendered_points: 10,
            ..Default::default()
        };

        assert_eq!(
            select(&points, &evaluation.mask, 1_000, 1_000, config).point_indices,
            select(&points, &evaluation.mask, 1_000, 1_000, config).point_indices
        );
    }

    #[test]
    fn point_reveal_respects_mask_and_viewport() {
        let points = points(4);
        let evaluation = evaluation(4, Some("keep"));
        let selection = select_points_for_reveal(
            &points,
            &evaluation.mask,
            F32Range::new(0.0, 2.1),
            F32Range::new(0.0, 2.1),
            100,
            100,
            PointRevealConfig::default(),
        )
        .unwrap();

        assert_eq!(selection.point_indices, vec![0, 2]);
        assert_eq!(selection.eligible_count, 2);
    }

    #[test]
    fn point_reveal_off_selects_no_points() {
        let points = points(4);
        let evaluation = evaluation(4, None);
        let selection = select(
            &points,
            &evaluation.mask,
            100,
            100,
            PointRevealConfig {
                mode: PointRevealMode::Off,
                ..Default::default()
            },
        );

        assert!(selection.point_indices.is_empty());
        assert_eq!(selection.eligible_count, 4);
        assert_eq!(selection.blend, 0.0);
    }

    #[test]
    fn point_projection_matches_density_coordinates() {
        let point = ScatterPointRecord {
            row_id: RowId(1),
            x: 2.5,
            y: 7.5,
            kind: ScatterPointKind::Unclassified,
        };

        assert_eq!(
            project_point_to_plot_fraction(
                &point,
                F32Range::new(0.0, 10.0),
                F32Range::new(0.0, 10.0),
            ),
            Some((0.25, 0.25))
        );
    }

    fn select(
        points: &[ScatterPointRecord],
        mask: &FilterMask,
        width: u32,
        height: u32,
        config: PointRevealConfig,
    ) -> PointRevealSelection {
        select_points_for_reveal(
            points,
            mask,
            F32Range::new(0.0, 1_000.0),
            F32Range::new(0.0, 1_000.0),
            width,
            height,
            config,
        )
        .unwrap()
    }

    fn points(count: usize) -> Vec<ScatterPointRecord> {
        (0..count)
            .map(|index| ScatterPointRecord {
                row_id: RowId(index as u64),
                x: index as f32,
                y: index as f32,
                kind: ScatterPointKind::Unclassified,
            })
            .collect()
    }

    fn evaluation(count: usize, included_value: Option<&str>) -> rawscope_data::FilterEvaluation {
        let source = LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "cohort".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: (0..count)
                .map(|index| LoadedSourceRow {
                    row_id: RowId(index as u64),
                    values: vec![if index % 2 == 0 { "keep" } else { "drop" }.into()],
                })
                .collect(),
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let mut filters = FilterSet::default();
        if let Some(value) = included_value {
            filters.replace_for_column(DatasetFilter::Categories {
                column_name: "cohort".into(),
                included_values: vec![value.into()],
                include_missing: false,
            });
        }
        evaluate_filters(&source, &catalog, &filters).unwrap()
    }
}
