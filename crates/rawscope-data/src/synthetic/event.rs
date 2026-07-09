//! Deterministic synthetic event data for timeline-density testing.

use rawscope_core::{RowId, U64Range};

use crate::dataset::{DatasetIdentity, SyntheticDatasetMetadata};

use super::rng::SyntheticRng;

const DEFAULT_TIME_MIN: u64 = 0;
const DEFAULT_TIME_MAX: u64 = 1_000;
const DEFAULT_LANE_COUNT: u32 = 8;
const GAP_START: u64 = 220;
const GAP_END: u64 = 280;
const SPIKE_START: u64 = 460;
const SPIKE_END: u64 = 500;
const STALE_LANE_END: u64 = 120;
const ANOMALY_START: u64 = 760;
const ANOMALY_END: u64 = 820;

/// The type of deterministic synthetic event pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticEventType {
    Background,
    Spike,
    StaleLane,
    HighValueBand,
}

/// One synthetic event for timeline-density testing.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticEventRecord {
    pub row_id: RowId,
    pub timestamp: u64,
    pub lane: u32,
    pub value: f32,
    pub event_type: SyntheticEventType,
}

/// Configuration for deterministic synthetic event generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticEventConfig {
    pub seed: u64,
    pub row_count: usize,
    pub time_range: U64Range,
    pub lane_count: u32,
}

impl SyntheticEventConfig {
    /// Creates an event config with the default synthetic timeline.
    pub fn new(seed: u64, row_count: usize) -> Self {
        Self {
            seed,
            row_count,
            time_range: U64Range::new(DEFAULT_TIME_MIN, DEFAULT_TIME_MAX),
            lane_count: DEFAULT_LANE_COUNT,
        }
    }
}

/// A deterministic synthetic event dataset plus generation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticEventDataset {
    pub identity: DatasetIdentity,
    pub metadata: SyntheticDatasetMetadata,
    pub time_range: U64Range,
    pub lane_count: u32,
    pub events: Vec<SyntheticEventRecord>,
}

/// Generates synthetic events with spike, gap, stale-lane, and anomaly patterns.
pub fn generate_synthetic_events(config: SyntheticEventConfig) -> SyntheticEventDataset {
    assert!(config.lane_count > 1, "lane_count must be greater than one");

    let mut rng = SyntheticRng::new(config.seed);
    let spike_count = usize::max(1, config.row_count / 4);
    let stale_lane_count = usize::max(1, config.row_count / 10);
    let anomaly_count = usize::max(1, config.row_count / 8);
    let background_count = config
        .row_count
        .saturating_sub(spike_count + stale_lane_count + anomaly_count);
    let stale_lane = config.lane_count - 1;
    let anomaly_lane_limit = u32::min(config.lane_count.saturating_sub(1), 3);

    let mut events = Vec::with_capacity(config.row_count);

    let background_rows = 0..background_count;
    for row_index in background_rows {
        events.push(SyntheticEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_time_outside_gap(&mut rng, config.time_range),
            lane: rng.u32_in_range(0, stale_lane),
            value: rng.f32_in_range(5.0, 25.0),
            event_type: SyntheticEventType::Background,
        });
    }

    let spike_start_row = background_count;
    let spike_end_row = spike_start_row + spike_count;
    let spike_rows = spike_start_row..spike_end_row;
    for row_index in spike_rows {
        events.push(SyntheticEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_window_time(&mut rng, config.time_range, SPIKE_START, SPIKE_END),
            lane: rng.u32_in_range(0, stale_lane),
            value: rng.f32_in_range(15.0, 35.0),
            event_type: SyntheticEventType::Spike,
        });
    }

    let stale_lane_start_row = spike_end_row;
    let stale_lane_end_row = stale_lane_start_row + stale_lane_count;
    let stale_lane_rows = stale_lane_start_row..stale_lane_end_row;
    for row_index in stale_lane_rows {
        events.push(SyntheticEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_window_time(
                &mut rng,
                config.time_range,
                config.time_range.min,
                STALE_LANE_END,
            ),
            lane: stale_lane,
            value: rng.f32_in_range(4.0, 18.0),
            event_type: SyntheticEventType::StaleLane,
        });
    }

    let anomaly_rows = stale_lane_end_row..config.row_count;
    for row_index in anomaly_rows {
        let anomaly_lanes_available = anomaly_lane_limit > 0;
        let lane = if anomaly_lanes_available {
            rng.u32_in_range(0, anomaly_lane_limit + 1)
        } else {
            0
        };
        events.push(SyntheticEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_window_time(&mut rng, config.time_range, ANOMALY_START, ANOMALY_END),
            lane,
            value: rng.f32_in_range(80.0, 120.0),
            event_type: SyntheticEventType::HighValueBand,
        });
    }

    SyntheticEventDataset {
        identity: DatasetIdentity::synthetic_timeline(
            config.seed,
            config.row_count,
            config.lane_count,
        ),
        metadata: SyntheticDatasetMetadata::new(config.seed, config.row_count),
        time_range: config.time_range,
        lane_count: config.lane_count,
        events,
    }
}

fn sample_time_outside_gap(rng: &mut SyntheticRng, time_range: U64Range) -> u64 {
    let early_min = time_range.min;
    let early_max = u64::min(GAP_START.saturating_sub(1), time_range.max);
    let late_min = u64::max(GAP_END.saturating_add(1), time_range.min);
    let early_span = early_max.saturating_sub(early_min) + 1;
    let late_span = if late_min > time_range.max {
        0
    } else {
        time_range.max.saturating_sub(late_min) + 1
    };
    let has_no_early_gap_head = early_span == 0;
    let has_no_late_gap_tail = late_span == 0;

    if has_no_early_gap_head {
        return rng.u64_in_range(late_min, time_range.max + 1);
    }

    if has_no_late_gap_tail {
        return rng.u64_in_range(early_min, early_max + 1);
    }

    let total_span = early_span + late_span;
    let use_early_gap_head = rng.u64_in_range(0, total_span) < early_span;
    if use_early_gap_head {
        rng.u64_in_range(early_min, early_max + 1)
    } else {
        rng.u64_in_range(late_min, time_range.max + 1)
    }
}

fn sample_window_time(
    rng: &mut SyntheticRng,
    time_range: U64Range,
    window_min: u64,
    window_max: u64,
) -> u64 {
    let min = u64::max(time_range.min, window_min);
    let max = u64::min(time_range.max, window_max);
    rng.u64_in_range(min, max + 1)
}
