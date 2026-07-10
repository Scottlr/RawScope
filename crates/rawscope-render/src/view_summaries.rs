//! CPU summary tracks for density view context.

use rawscope_core::{F32Range, U64Range};
use rawscope_data::{FilterMask, ScatterPointRecord, TimelineEventRecord};

use crate::MaskAlignmentError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryBin {
    pub index: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterMarginalSummary {
    pub x_bins: Vec<SummaryBin>,
    pub y_bins: Vec<SummaryBin>,
    pub max_x_count: u32,
    pub max_y_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineMarginalSummary {
    pub time_bins: Vec<SummaryBin>,
    pub lane_bins: Vec<SummaryBin>,
    pub max_time_count: u32,
    pub max_lane_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineOverviewWindow {
    pub start_fraction: f32,
    pub end_fraction: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOverviewSummary {
    pub full_time_range: U64Range,
    pub current_time_range: U64Range,
    pub time_bins: Vec<SummaryBin>,
    pub max_time_count: u32,
}

impl TimelineOverviewSummary {
    pub fn current_window(&self) -> TimelineOverviewWindow {
        let full_span = self.full_time_range.span() as f32;
        let start_fraction = self
            .current_time_range
            .min
            .saturating_sub(self.full_time_range.min) as f32
            / full_span;
        let end_fraction = self
            .current_time_range
            .max
            .saturating_sub(self.full_time_range.min) as f32
            / full_span;

        TimelineOverviewWindow {
            start_fraction: start_fraction.clamp(0.0, 1.0),
            end_fraction: end_fraction.clamp(0.0, 1.0),
        }
    }
}

pub fn scatter_marginal_summary(
    points: &[ScatterPointRecord],
    x_range: F32Range,
    y_range: F32Range,
    x_bin_count: u32,
    y_bin_count: u32,
) -> ScatterMarginalSummary {
    assert!(x_bin_count > 0, "x_bin_count must be positive");
    assert!(y_bin_count > 0, "y_bin_count must be positive");

    let mut x_counts = vec![0u32; x_bin_count as usize];
    let mut y_counts = vec![0u32; y_bin_count as usize];

    for point in points {
        let Some(x_bin) = bin_f32(point.x, x_range, x_bin_count) else {
            continue;
        };
        let Some(y_bin) = bin_f32(point.y, y_range, y_bin_count) else {
            continue;
        };

        bump_count(&mut x_counts, x_bin as usize);
        bump_count(&mut y_counts, y_bin as usize);
    }

    let max_x_count = max_count(&x_counts);
    let max_y_count = max_count(&y_counts);

    ScatterMarginalSummary {
        x_bins: bins_from_counts(&x_counts),
        y_bins: bins_from_counts(&y_counts),
        max_x_count,
        max_y_count,
    }
}

pub fn scatter_marginal_summary_masked(
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    x_range: F32Range,
    y_range: F32Range,
    x_bin_count: u32,
    y_bin_count: u32,
) -> Result<ScatterMarginalSummary, MaskAlignmentError> {
    MaskAlignmentError::require(points.len(), mask.len())?;
    assert!(x_bin_count > 0, "x_bin_count must be positive");
    assert!(y_bin_count > 0, "y_bin_count must be positive");
    let mut x_counts = vec![0u32; x_bin_count as usize];
    let mut y_counts = vec![0u32; y_bin_count as usize];
    for (point, included) in points.iter().zip(mask.as_gpu_u32_slice()) {
        if *included == 0 {
            continue;
        }
        let (Some(x_bin), Some(y_bin)) = (
            bin_f32(point.x, x_range, x_bin_count),
            bin_f32(point.y, y_range, y_bin_count),
        ) else {
            continue;
        };
        bump_count(&mut x_counts, x_bin as usize);
        bump_count(&mut y_counts, y_bin as usize);
    }
    Ok(ScatterMarginalSummary {
        max_x_count: max_count(&x_counts),
        max_y_count: max_count(&y_counts),
        x_bins: bins_from_counts(&x_counts),
        y_bins: bins_from_counts(&y_counts),
    })
}

pub fn timeline_marginal_summary(
    events: &[TimelineEventRecord],
    time_range: U64Range,
    lane_count: u32,
    time_bin_count: u32,
) -> TimelineMarginalSummary {
    assert!(lane_count > 0, "lane_count must be positive");
    assert!(time_bin_count > 0, "time_bin_count must be positive");

    let mut time_counts = vec![0u32; time_bin_count as usize];
    let mut lane_counts = vec![0u32; lane_count as usize];

    for event in events {
        let Some(time_bin) = bin_u64(event.timestamp, time_range, time_bin_count) else {
            continue;
        };
        let Some(lane_bin) = bin_lane(event.lane, lane_count) else {
            continue;
        };

        bump_count(&mut time_counts, time_bin as usize);
        bump_count(&mut lane_counts, lane_bin as usize);
    }

    let max_time_count = max_count(&time_counts);
    let max_lane_count = max_count(&lane_counts);

    TimelineMarginalSummary {
        time_bins: bins_from_counts(&time_counts),
        lane_bins: bins_from_counts(&lane_counts),
        max_time_count,
        max_lane_count,
    }
}

pub fn timeline_overview_summary(
    events: &[TimelineEventRecord],
    full_time_range: U64Range,
    current_time_range: U64Range,
    time_bin_count: u32,
) -> TimelineOverviewSummary {
    assert!(time_bin_count > 0, "time_bin_count must be positive");

    let mut time_counts = vec![0u32; time_bin_count as usize];

    for event in events {
        let Some(time_bin) = bin_u64(event.timestamp, full_time_range, time_bin_count) else {
            continue;
        };

        bump_count(&mut time_counts, time_bin as usize);
    }

    let max_time_count = max_count(&time_counts);

    TimelineOverviewSummary {
        full_time_range,
        current_time_range,
        time_bins: bins_from_counts(&time_counts),
        max_time_count,
    }
}

fn bump_count(counts: &mut [u32], index: usize) {
    counts[index] = counts[index].saturating_add(1);
}

fn bins_from_counts(counts: &[u32]) -> Vec<SummaryBin> {
    counts
        .iter()
        .copied()
        .enumerate()
        .map(|(index, count)| SummaryBin {
            index: index as u32,
            count,
        })
        .collect()
}

fn max_count(counts: &[u32]) -> u32 {
    counts.iter().copied().max().unwrap_or(0)
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

fn bin_lane(lane: u32, lane_count: u32) -> Option<u32> {
    if lane >= lane_count {
        return None;
    }

    Some(lane)
}

#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, RowId, U64Range};
    use rawscope_data::{
        ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
    };

    use super::{
        scatter_marginal_summary, timeline_marginal_summary, timeline_overview_summary, SummaryBin,
    };

    fn scatter_point(row_id: u64, x: f32, y: f32) -> ScatterPointRecord {
        ScatterPointRecord {
            row_id: RowId(row_id),
            x,
            y,
            kind: ScatterPointKind::Unclassified,
        }
    }

    fn timeline_event(row_id: u64, timestamp: u64, lane: u32) -> TimelineEventRecord {
        TimelineEventRecord {
            row_id: RowId(row_id),
            timestamp,
            lane,
            value: 0.0,
            kind: TimelineEventKind::Unclassified,
        }
    }

    #[test]
    fn scatter_marginals_count_points_on_expected_axes() {
        let points = [
            scatter_point(1, 0.0, 0.0),
            scatter_point(2, 1.999, 2.0),
            scatter_point(3, 2.0, 4.0),
            scatter_point(4, 8.0, 8.0),
        ];

        let summary = scatter_marginal_summary(
            &points,
            F32Range::new(0.0, 8.0),
            F32Range::new(0.0, 8.0),
            4,
            4,
        );

        assert_eq!(
            summary.x_bins,
            vec![
                SummaryBin { index: 0, count: 2 },
                SummaryBin { index: 1, count: 1 },
                SummaryBin { index: 2, count: 0 },
                SummaryBin { index: 3, count: 1 },
            ]
        );
        assert_eq!(
            summary.y_bins,
            vec![
                SummaryBin { index: 0, count: 1 },
                SummaryBin { index: 1, count: 1 },
                SummaryBin { index: 2, count: 1 },
                SummaryBin { index: 3, count: 1 },
            ]
        );
        assert_eq!(summary.max_x_count, 2);
        assert_eq!(summary.max_y_count, 1);
    }

    #[test]
    fn timeline_marginals_count_events_by_time_and_lane() {
        let events = [
            timeline_event(1, 100, 0),
            timeline_event(2, 199, 1),
            timeline_event(3, 250, 1),
            timeline_event(4, 499, 2),
            timeline_event(5, 500, 2),
        ];

        let summary = timeline_marginal_summary(&events, U64Range::new(100, 500), 3, 4);

        assert_eq!(
            summary.time_bins,
            vec![
                SummaryBin { index: 0, count: 2 },
                SummaryBin { index: 1, count: 1 },
                SummaryBin { index: 2, count: 0 },
                SummaryBin { index: 3, count: 2 },
            ]
        );
        assert_eq!(
            summary.lane_bins,
            vec![
                SummaryBin { index: 0, count: 1 },
                SummaryBin { index: 1, count: 2 },
                SummaryBin { index: 2, count: 2 },
            ]
        );
        assert_eq!(summary.max_time_count, 2);
        assert_eq!(summary.max_lane_count, 2);
    }

    #[test]
    fn timeline_overview_window_projects_current_range() {
        let events = [
            timeline_event(1, 100, 0),
            timeline_event(2, 199, 0),
            timeline_event(3, 200, 1),
            timeline_event(4, 349, 1),
            timeline_event(5, 500, 2),
        ];

        let summary =
            timeline_overview_summary(&events, U64Range::new(100, 500), U64Range::new(200, 400), 4);

        assert_eq!(
            summary.time_bins,
            vec![
                SummaryBin { index: 0, count: 2 },
                SummaryBin { index: 1, count: 1 },
                SummaryBin { index: 2, count: 1 },
                SummaryBin { index: 3, count: 1 },
            ]
        );
        assert_eq!(summary.max_time_count, 2);
        assert_eq!(summary.full_time_range, U64Range::new(100, 500));
        assert_eq!(summary.current_time_range, U64Range::new(200, 400));

        let window = summary.current_window();
        assert!((window.start_fraction - 0.25).abs() < f32::EPSILON);
        assert!((window.end_fraction - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn summaries_ignore_records_outside_visible_range() {
        let scatter_points = [
            scatter_point(1, 1.0, 1.0),
            scatter_point(2, -1.0, 1.0),
            scatter_point(3, 1.0, 9.0),
        ];
        let scatter_summary = scatter_marginal_summary(
            &scatter_points,
            F32Range::new(0.0, 4.0),
            F32Range::new(0.0, 4.0),
            4,
            4,
        );

        assert_eq!(scatter_summary.max_x_count, 1);
        assert_eq!(scatter_summary.max_y_count, 1);

        let events = [
            timeline_event(1, 200, 1),
            timeline_event(2, 99, 1),
            timeline_event(3, 201, 4),
        ];
        let timeline_summary = timeline_marginal_summary(&events, U64Range::new(100, 300), 3, 4);
        let overview_summary =
            timeline_overview_summary(&events, U64Range::new(100, 300), U64Range::new(120, 280), 4);

        assert_eq!(
            timeline_summary.time_bins,
            vec![
                SummaryBin { index: 0, count: 0 },
                SummaryBin { index: 1, count: 0 },
                SummaryBin { index: 2, count: 1 },
                SummaryBin { index: 3, count: 0 },
            ]
        );
        assert_eq!(timeline_summary.max_lane_count, 1);
        assert_eq!(
            overview_summary.time_bins,
            vec![
                SummaryBin { index: 0, count: 0 },
                SummaryBin { index: 1, count: 0 },
                SummaryBin { index: 2, count: 2 },
                SummaryBin { index: 3, count: 0 },
            ]
        );
    }
}
