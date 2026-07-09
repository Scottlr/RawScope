//! Deterministic synthetic point data for scatter-density testing.

use rawscope_core::{F32Range, RowId};

use crate::dataset::{DatasetIdentity, SyntheticDatasetMetadata};

use super::rng::SyntheticRng;

const DEFAULT_POINT_MIN: f32 = 0.0;
const DEFAULT_POINT_MAX: f32 = 100.0;
const CLUSTER_X_CENTER: f32 = 28.0;
const CLUSTER_Y_CENTER: f32 = 72.0;
const CLUSTER_SPREAD: f32 = 10.0;
const BACKGROUND_SHARE_DENOMINATOR: usize = 4;
const OUTLIER_SHARE_DENOMINATOR: usize = 20;

/// The synthetic point category for a generated row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticPointCategory {
    Cluster,
    Background,
    Outlier,
}

/// One synthetic point for scatter-density testing.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticPointRecord {
    pub row_id: RowId,
    pub x: f32,
    pub y: f32,
    pub category: SyntheticPointCategory,
}

/// Configuration for deterministic synthetic point generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticPointConfig {
    pub seed: u64,
    pub row_count: usize,
    pub x_range: F32Range,
    pub y_range: F32Range,
}

impl SyntheticPointConfig {
    /// Creates a point config with the default synthetic range.
    pub fn new(seed: u64, row_count: usize) -> Self {
        Self {
            seed,
            row_count,
            x_range: F32Range::new(DEFAULT_POINT_MIN, DEFAULT_POINT_MAX),
            y_range: F32Range::new(DEFAULT_POINT_MIN, DEFAULT_POINT_MAX),
        }
    }
}

/// A deterministic synthetic point dataset plus generation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticPointDataset {
    pub identity: DatasetIdentity,
    pub metadata: SyntheticDatasetMetadata,
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub points: Vec<SyntheticPointRecord>,
}

/// Generates synthetic points with one dense cluster plus sparse outliers.
pub fn generate_synthetic_points(config: SyntheticPointConfig) -> SyntheticPointDataset {
    let mut rng = SyntheticRng::new(config.seed);
    let background_count = config.row_count / BACKGROUND_SHARE_DENOMINATOR;
    let outlier_count = usize::max(1, config.row_count / OUTLIER_SHARE_DENOMINATOR);
    let cluster_count = config
        .row_count
        .saturating_sub(background_count + outlier_count);

    let mut points = Vec::with_capacity(config.row_count);

    let cluster_rows = 0..cluster_count;
    for row_index in cluster_rows {
        points.push(SyntheticPointRecord {
            row_id: RowId(row_index as u64),
            x: sample_cluster_value(&mut rng, config.x_range, CLUSTER_X_CENTER, CLUSTER_SPREAD),
            y: sample_cluster_value(&mut rng, config.y_range, CLUSTER_Y_CENTER, CLUSTER_SPREAD),
            category: SyntheticPointCategory::Cluster,
        });
    }

    let background_start = cluster_count;
    let background_end = background_start + background_count;
    let background_rows = background_start..background_end;
    for row_index in background_rows {
        points.push(SyntheticPointRecord {
            row_id: RowId(row_index as u64),
            x: rng.f32_in_range(config.x_range.min, config.x_range.max),
            y: rng.f32_in_range(config.y_range.min, config.y_range.max),
            category: SyntheticPointCategory::Background,
        });
    }

    let outlier_rows = background_end..config.row_count;
    for row_index in outlier_rows {
        let (x, y) = sample_outlier(&mut rng, config.x_range, config.y_range);
        points.push(SyntheticPointRecord {
            row_id: RowId(row_index as u64),
            x,
            y,
            category: SyntheticPointCategory::Outlier,
        });
    }

    SyntheticPointDataset {
        identity: DatasetIdentity::synthetic_scatter(config.seed, config.row_count),
        metadata: SyntheticDatasetMetadata::new(config.seed, config.row_count),
        x_range: config.x_range,
        y_range: config.y_range,
        points,
    }
}

fn sample_cluster_value(rng: &mut SyntheticRng, range: F32Range, center: f32, spread: f32) -> f32 {
    let normalized = (rng.next_unit_f32() + rng.next_unit_f32() + rng.next_unit_f32()) / 3.0;
    let offset = (normalized - 0.5) * spread;
    (center + offset).clamp(range.min, range.max)
}

fn sample_outlier(rng: &mut SyntheticRng, x_range: F32Range, y_range: F32Range) -> (f32, f32) {
    let edge_jitter_x = x_range.span() * 0.02;
    let edge_jitter_y = y_range.span() * 0.02;

    match rng.u32_in_range(0, 4) {
        0 => (
            x_range.min + rng.f32_in_range(0.0, edge_jitter_x),
            y_range.min + rng.f32_in_range(0.0, edge_jitter_y),
        ),
        1 => (
            x_range.max - rng.f32_in_range(0.0, edge_jitter_x),
            y_range.min + rng.f32_in_range(0.0, edge_jitter_y),
        ),
        2 => (
            x_range.min + rng.f32_in_range(0.0, edge_jitter_x),
            y_range.max - rng.f32_in_range(0.0, edge_jitter_y),
        ),
        _ => (
            x_range.max - rng.f32_in_range(0.0, edge_jitter_x),
            y_range.max - rng.f32_in_range(0.0, edge_jitter_y),
        ),
    }
}
