//! Bounded aggregate overviews used by evidence and workbench context.

use std::{error::Error, fmt};

use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{ScatterPointRecord, TimelineEventRecord};

/// Configuration for a bounded aggregate cache overview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateCacheConfig {
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_row_ids_per_bin: usize,
}

/// One aggregate bin with a full count and a bounded row-id sample.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AggregateBinSample {
    pub count: u32,
    pub row_ids: Vec<RowId>,
}

/// Full scatter-view aggregate overview with per-bin row samples.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterAggregateOverview {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_bin_count: u32,
    pub bins: Vec<AggregateBinSample>,
}

/// Full timeline-view aggregate overview with per-bin row samples.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineAggregateOverview {
    pub time_range: U64Range,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub max_bin_count: u32,
    pub bins: Vec<AggregateBinSample>,
}

/// Errors returned while building a bounded aggregate overview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateCacheError {
    EmptyGrid,
    EmptyTimelineLanes,
}

impl fmt::Display for AggregateCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrid => write!(f, "aggregate grid dimensions must be positive"),
            Self::EmptyTimelineLanes => {
                write!(f, "timeline lane count must be positive")
            }
        }
    }
}

impl Error for AggregateCacheError {}

/// Builds a scatter aggregate overview with deterministic row-id sampling.
pub fn scatter_aggregate_overview(
    points: &[ScatterPointRecord],
    x_range: F32Range,
    y_range: F32Range,
    config: AggregateCacheConfig,
) -> Result<ScatterAggregateOverview, AggregateCacheError> {
    validate_grid(config.grid_width, config.grid_height)?;

    let mut bins = empty_bins(
        config.grid_width,
        config.grid_height,
        config.max_row_ids_per_bin,
    );
    for point in points {
        let Some(x_bin) = bin_f32(point.x, x_range, config.grid_width) else {
            continue;
        };
        let Some(y_bin) = bin_f32(point.y, y_range, config.grid_height) else {
            continue;
        };

        let bin_index = bin_index(config.grid_width, x_bin, y_bin);
        record_bin_sample(
            &mut bins[bin_index],
            point.row_id,
            config.max_row_ids_per_bin,
        );
    }

    let max_bin_count = bins.iter().map(|bin| bin.count).max().unwrap_or(0);
    Ok(ScatterAggregateOverview {
        x_range,
        y_range,
        grid_width: config.grid_width,
        grid_height: config.grid_height,
        max_bin_count,
        bins,
    })
}

/// Builds a timeline aggregate overview with deterministic row-id sampling.
pub fn timeline_aggregate_overview(
    events: &[TimelineEventRecord],
    time_range: U64Range,
    lane_count: u32,
    config: AggregateCacheConfig,
) -> Result<TimelineAggregateOverview, AggregateCacheError> {
    if lane_count == 0 {
        return Err(AggregateCacheError::EmptyTimelineLanes);
    }
    validate_grid(config.grid_width, config.grid_height)?;

    let mut bins = empty_bins(
        config.grid_width,
        config.grid_height,
        config.max_row_ids_per_bin,
    );
    for event in events {
        let Some(x_bin) = bin_u64(event.timestamp, time_range, config.grid_width) else {
            continue;
        };
        let Some(y_bin) = bin_lane(event.lane, lane_count, config.grid_height) else {
            continue;
        };

        let bin_index = bin_index(config.grid_width, x_bin, y_bin);
        record_bin_sample(
            &mut bins[bin_index],
            event.row_id,
            config.max_row_ids_per_bin,
        );
    }

    let max_bin_count = bins.iter().map(|bin| bin.count).max().unwrap_or(0);
    Ok(TimelineAggregateOverview {
        time_range,
        lane_count,
        grid_width: config.grid_width,
        grid_height: config.grid_height,
        max_bin_count,
        bins,
    })
}

fn validate_grid(grid_width: u32, grid_height: u32) -> Result<(), AggregateCacheError> {
    if grid_width == 0 || grid_height == 0 {
        return Err(AggregateCacheError::EmptyGrid);
    }

    Ok(())
}

fn empty_bins(
    grid_width: u32,
    grid_height: u32,
    max_row_ids_per_bin: usize,
) -> Vec<AggregateBinSample> {
    let bin_count = (grid_width as usize) * (grid_height as usize);
    (0..bin_count)
        .map(|_| AggregateBinSample {
            count: 0,
            row_ids: Vec::with_capacity(max_row_ids_per_bin),
        })
        .collect()
}

fn record_bin_sample(bin: &mut AggregateBinSample, row_id: RowId, max_row_ids_per_bin: usize) {
    bin.count = bin.count.saturating_add(1);
    if bin.row_ids.len() < max_row_ids_per_bin {
        bin.row_ids.push(row_id);
    }
}

fn bin_index(grid_width: u32, x_bin: u32, y_bin: u32) -> usize {
    (y_bin as usize) * (grid_width as usize) + (x_bin as usize)
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
