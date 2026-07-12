//! CPU reference density implementations for deterministic testing.

use rawscope_core::{DensityGrid, F32Range, GridSize, U64Range};
use rawscope_data::{ScatterPointRecord, TimelineEventRecord};

/// Bins scatter points into a simple scatter-density grid.
pub fn scatter_density(
    points: &[ScatterPointRecord],
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> DensityGrid {
    let mut grid = DensityGrid::new(GridSize::new(width, height));

    for point in points {
        let Some(x_bin) = bin_f32(point.x, x_range, width) else {
            continue;
        };
        let Some(y_bin) = bin_f32(point.y, y_range, height) else {
            continue;
        };

        let _ = grid.bin_mut(x_bin, y_bin).push(point.row_id);
    }

    grid
}

/// Bins timeline events into a density grid.
pub fn timeline_density(
    events: &[TimelineEventRecord],
    time_range: U64Range,
    lane_count: u32,
    width: u32,
    height: u32,
) -> DensityGrid {
    assert!(lane_count > 0, "lane_count must be positive");

    let mut grid = DensityGrid::new(GridSize::new(width, height));

    for event in events {
        let Some(x_bin) = bin_u64(event.timestamp, time_range, width) else {
            continue;
        };
        let Some(y_bin) = bin_lane(event.lane, lane_count, height) else {
            continue;
        };

        let _ = grid.bin_mut(x_bin, y_bin).push(event.row_id);
    }

    grid
}

fn bin_f32(value: f32, range: F32Range, bin_count: u32) -> Option<u32> {
    if !range.contains(value) {
        return None;
    }

    if value == range.max {
        return Some(bin_count - 1);
    }

    let normalized = (value - range.min) / range.span();
    let raw_bin = (normalized * bin_count as f32).floor() as u32;
    Some(raw_bin.min(bin_count - 1))
}

fn bin_u64(value: u64, range: U64Range, bin_count: u32) -> Option<u32> {
    if !range.contains(value) {
        return None;
    }

    if value == range.max {
        return Some(bin_count - 1);
    }

    let normalized = (value - range.min) as f64 / range.span() as f64;
    let raw_bin = (normalized * bin_count as f64).floor() as u32;
    Some(raw_bin.min(bin_count - 1))
}

fn bin_lane(lane: u32, lane_count: u32, height: u32) -> Option<u32> {
    if lane >= lane_count {
        return None;
    }

    let raw_bin = ((lane as u64) * (height as u64) / (lane_count as u64)) as u32;
    Some(raw_bin.min(height - 1))
}
