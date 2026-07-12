//! Deterministic synthetic event data for timeline-density testing.

use std::{error::Error, fmt};

use rawscope_core::{RowId, U64Range};

use crate::dataset::{DatasetIdentity, SyntheticDatasetMetadata};
use crate::{TimelineEventKind, TimelineEventRecord};

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

    pub fn validate(self) -> Result<ValidatedSyntheticEventConfig, SyntheticEventConfigError> {
        if self.lane_count <= 1 {
            return Err(SyntheticEventConfigError::LaneCountTooSmall {
                actual: self.lane_count,
            });
        }

        let spike_count = self.row_count / 4;
        let stale_lane_count = self.row_count / 10;
        let anomaly_count = self.row_count / 8;
        if self.row_count > 0
            && !overlaps(self.time_range, GAP_START, GAP_END)
            && !has_outside_gap_value(self.time_range)
        {
            return Err(SyntheticEventConfigError::NoBackgroundWindow);
        }
        if spike_count > 0 && !overlaps(self.time_range, SPIKE_START, SPIKE_END) {
            return Err(SyntheticEventConfigError::WindowOutsideRange {
                pattern: SyntheticEventType::Spike,
            });
        }
        if stale_lane_count > 0 && !overlaps(self.time_range, self.time_range.min, STALE_LANE_END) {
            return Err(SyntheticEventConfigError::WindowOutsideRange {
                pattern: SyntheticEventType::StaleLane,
            });
        }
        if anomaly_count > 0 && !overlaps(self.time_range, ANOMALY_START, ANOMALY_END) {
            return Err(SyntheticEventConfigError::WindowOutsideRange {
                pattern: SyntheticEventType::HighValueBand,
            });
        }
        Ok(ValidatedSyntheticEventConfig(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedSyntheticEventConfig(SyntheticEventConfig);

impl ValidatedSyntheticEventConfig {
    pub const fn config(self) -> SyntheticEventConfig {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticEventConfigError {
    LaneCountTooSmall { actual: u32 },
    NoBackgroundWindow,
    WindowOutsideRange { pattern: SyntheticEventType },
}

impl fmt::Display for SyntheticEventConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaneCountTooSmall { actual } => {
                write!(
                    formatter,
                    "synthetic timeline requires at least two lanes, got {actual}"
                )
            }
            Self::NoBackgroundWindow => {
                write!(
                    formatter,
                    "synthetic time range contains no value outside the gap"
                )
            }
            Self::WindowOutsideRange { pattern } => {
                write!(
                    formatter,
                    "synthetic {pattern:?} window does not overlap the configured time range"
                )
            }
        }
    }
}

impl Error for SyntheticEventConfigError {}

/// A deterministic synthetic event dataset plus generation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticEventDataset {
    pub identity: DatasetIdentity,
    pub metadata: SyntheticDatasetMetadata,
    pub time_range: U64Range,
    pub lane_count: u32,
    pub events: Vec<TimelineEventRecord>,
}

/// Validates and generates synthetic events with typed configuration errors.
pub fn try_generate_synthetic_events(
    config: SyntheticEventConfig,
) -> Result<SyntheticEventDataset, SyntheticEventConfigError> {
    let validated = config.validate()?;
    Ok(generate_validated_synthetic_events(validated))
}

/// Compatibility wrapper for existing known-valid demo configurations.
pub fn generate_synthetic_events(config: SyntheticEventConfig) -> SyntheticEventDataset {
    try_generate_synthetic_events(config).expect("validated synthetic event configuration")
}

fn generate_validated_synthetic_events(
    validated: ValidatedSyntheticEventConfig,
) -> SyntheticEventDataset {
    let config = validated.config();

    let mut rng = SyntheticRng::new(config.seed);
    let spike_count = config.row_count / 4;
    let stale_lane_count = config.row_count / 10;
    let anomaly_count = config.row_count / 8;
    let pattern_count = spike_count
        .saturating_add(stale_lane_count)
        .saturating_add(anomaly_count);
    let background_count = config.row_count.saturating_sub(pattern_count);
    let stale_lane = config.lane_count - 1;
    let anomaly_lane_limit = u32::min(config.lane_count.saturating_sub(1), 3);

    let mut events = Vec::with_capacity(config.row_count);

    let background_rows = 0..background_count;
    for row_index in background_rows {
        events.push(TimelineEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_time_outside_gap(&mut rng, config.time_range),
            lane: rng.u32_in_range(0, stale_lane),
            value: rng.f32_in_range(5.0, 25.0),
            kind: TimelineEventKind::Synthetic(SyntheticEventType::Background),
        });
    }

    let spike_start_row = background_count;
    let spike_end_row = spike_start_row + spike_count;
    let spike_rows = spike_start_row..spike_end_row;
    for row_index in spike_rows {
        events.push(TimelineEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_window_time(&mut rng, config.time_range, SPIKE_START, SPIKE_END),
            lane: rng.u32_in_range(0, stale_lane),
            value: rng.f32_in_range(15.0, 35.0),
            kind: TimelineEventKind::Synthetic(SyntheticEventType::Spike),
        });
    }

    let stale_lane_start_row = spike_end_row;
    let stale_lane_end_row = stale_lane_start_row + stale_lane_count;
    let stale_lane_rows = stale_lane_start_row..stale_lane_end_row;
    for row_index in stale_lane_rows {
        events.push(TimelineEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_window_time(
                &mut rng,
                config.time_range,
                config.time_range.min,
                STALE_LANE_END,
            ),
            lane: stale_lane,
            value: rng.f32_in_range(4.0, 18.0),
            kind: TimelineEventKind::Synthetic(SyntheticEventType::StaleLane),
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
        events.push(TimelineEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: sample_window_time(&mut rng, config.time_range, ANOMALY_START, ANOMALY_END),
            lane,
            value: rng.f32_in_range(80.0, 120.0),
            kind: TimelineEventKind::Synthetic(SyntheticEventType::HighValueBand),
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
        return sample_inclusive(rng, late_min, time_range.max);
    }

    if has_no_late_gap_tail {
        return sample_inclusive(rng, early_min, early_max);
    }

    let total_span = early_span + late_span;
    let use_early_gap_head = rng.u64_in_range(0, total_span) < early_span;
    if use_early_gap_head {
        sample_inclusive(rng, early_min, early_max)
    } else {
        sample_inclusive(rng, late_min, time_range.max)
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
    sample_inclusive(rng, min, max)
}

fn overlaps(range: U64Range, window_min: u64, window_max: u64) -> bool {
    range.min <= window_max && window_min <= range.max
}

fn has_outside_gap_value(range: U64Range) -> bool {
    range.min < GAP_START || range.max > GAP_END
}

fn sample_inclusive(rng: &mut SyntheticRng, min: u64, max: u64) -> u64 {
    if min == max {
        return min;
    }
    let max_exclusive = max.checked_add(1).unwrap_or(u64::MAX);
    rng.u64_in_range(min, max_exclusive)
}
