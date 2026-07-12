//! CPU reference density implementations for deterministic testing.

use std::num::NonZeroU32;

use rawscope_analysis::density::{bin_f32, bin_u64, BinIndex, BinPlacement};
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
        let Some(x_bin) = in_domain_bin(bin_f32(point.x, x_range, non_zero_bins(width))) else {
            continue;
        };
        let Some(y_bin) = in_domain_bin(bin_f32(point.y, y_range, non_zero_bins(height))) else {
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
        let Some(x_bin) = in_domain_bin(bin_u64(event.timestamp, time_range, non_zero_bins(width)))
        else {
            continue;
        };
        let Some(y_bin) = bin_lane(event.lane, lane_count, height) else {
            continue;
        };

        let _ = grid.bin_mut(x_bin, y_bin).push(event.row_id);
    }

    grid
}

fn non_zero_bins(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("density grid dimensions are validated by GridSize")
}

fn in_domain_bin(
    result: Result<BinPlacement, rawscope_analysis::density::BinningError>,
) -> Option<u32> {
    match result.ok()? {
        BinPlacement::InDomain(BinIndex(index)) => Some(index),
        BinPlacement::BeforeDomain | BinPlacement::AfterDomain => None,
    }
}

fn bin_lane(lane: u32, lane_count: u32, height: u32) -> Option<u32> {
    if lane >= lane_count {
        return None;
    }

    let raw_bin = ((lane as u64) * (height as u64) / (lane_count as u64)) as u32;
    Some(raw_bin.min(height - 1))
}
