//! Axis and lane context for density views.

use rawscope_core::{F32Range, U64Range};

const MIN_AXIS_TICK_COUNT: usize = 2;

/// Axis tick for scalar projections.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisTick {
    pub fraction: f32,
    pub label: String,
}

/// Single numeric axis projection with a display label and evenly distributed ticks.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericAxisContext {
    pub label: String,
    pub ticks: Vec<AxisTick>,
}

/// Scatter x/y axis projection owned by the render crate.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterAxesContext {
    pub x: NumericAxisContext,
    pub y: NumericAxisContext,
}

/// Per-lane overlay label for timeline rows.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineLaneLabel {
    pub lane: u32,
    pub fraction: f32,
    pub label: String,
}

/// Timeline axis projection owned by the render crate.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineAxesContext {
    pub time: NumericAxisContext,
    pub lanes: Vec<TimelineLaneLabel>,
}

/// Builds axis labels and tick labels for scatter x/y ranges.
pub fn scatter_axes_context(
    x_range: F32Range,
    y_range: F32Range,
    x_label: impl Into<String>,
    y_label: impl Into<String>,
    max_ticks: usize,
) -> ScatterAxesContext {
    let tick_count = clamp_tick_count(max_ticks);

    ScatterAxesContext {
        x: NumericAxisContext {
            label: x_label.into(),
            ticks: numeric_axis_ticks(x_range.min, x_range.max, tick_count, format_float_tick),
        },
        y: NumericAxisContext {
            label: y_label.into(),
            ticks: numeric_axis_ticks(y_range.min, y_range.max, tick_count, format_float_tick),
        },
    }
}

/// Builds time ticks and lane labels for timeline views.
pub fn timeline_axes_context(
    time_range: U64Range,
    lane_count: u32,
    lane_labels: &[String],
    max_time_ticks: usize,
    max_lane_labels: usize,
) -> TimelineAxesContext {
    let time_tick_count = clamp_tick_count(max_time_ticks);
    let lane_sample_count = clamp_tick_count(max_lane_labels).min(lane_count as usize);
    let lanes = sample_lane_labels(lane_count, lane_labels, lane_sample_count);

    TimelineAxesContext {
        time: NumericAxisContext {
            label: "time".to_string(),
            ticks: u64_axis_ticks(time_range.min, time_range.max, time_tick_count, |value| {
                value.to_string()
            }),
        },
        lanes,
    }
}

fn clamp_tick_count(requested: usize) -> usize {
    requested.max(MIN_AXIS_TICK_COUNT)
}

fn numeric_axis_ticks(
    min: f32,
    max: f32,
    tick_count: usize,
    format_label: impl Fn(f32) -> String,
) -> Vec<AxisTick> {
    if tick_count == 0 {
        return Vec::new();
    }

    let span = max - min;
    let denominator = tick_count as f32 - 1.0;

    (0..tick_count)
        .map(|tick_index| {
            let fraction = (tick_index as f32) / denominator;
            let value = min + (span * fraction);
            AxisTick {
                fraction: fraction.clamp(0.0, 1.0),
                label: format_label(value),
            }
        })
        .collect()
}

fn u64_axis_ticks(
    min: u64,
    max: u64,
    tick_count: usize,
    format_label: impl Fn(u64) -> String,
) -> Vec<AxisTick> {
    if tick_count == 0 {
        return Vec::new();
    }

    let span = (max - min) as f64;
    let last_index = tick_count - 1;
    let last_index_f64 = last_index as f64;

    (0..tick_count)
        .map(|tick_index| {
            let fraction = if last_index == 0 {
                0.0
            } else {
                (tick_index as f64) / last_index_f64
            };
            let scaled = (span * fraction).round();
            let value = if last_index == 0 {
                min
            } else {
                let upper = max as f64;
                let candidate = (min as f64 + scaled).clamp(min as f64, upper);
                candidate as u64
            };
            AxisTick {
                fraction: (fraction as f32).clamp(0.0, 1.0),
                label: format_label(value),
            }
        })
        .collect()
}

fn format_float_tick(value: f32) -> String {
    format!("{value:.1}")
}

fn sample_lane_labels(
    lane_count: u32,
    lane_labels: &[String],
    sample_count: usize,
) -> Vec<TimelineLaneLabel> {
    if lane_count == 0 || sample_count == 0 {
        return Vec::new();
    }

    let last_lane = lane_count - 1;
    let mut labels = Vec::with_capacity(sample_count);

    for sample_index in 0..sample_count {
        let lane = if sample_count == 1 {
            0
        } else {
            (last_lane as usize * sample_index) / (sample_count - 1)
        } as u32;
        labels.push(TimelineLaneLabel {
            lane,
            fraction: (lane as f32 + 0.5) / lane_count as f32,
            label: lane_label_text(lane_labels, lane),
        });
    }

    labels
}

fn lane_label_text(lane_labels: &[String], lane: u32) -> String {
    lane_labels
        .get(lane as usize)
        .filter(|label| !label.is_empty())
        .cloned()
        .unwrap_or_else(|| format!("lane-{lane}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scatter_axis_ticks_are_deterministic() {
        let first = scatter_axes_context(
            F32Range::new(0.0, 100.0),
            F32Range::new(50.0, 150.0),
            "x",
            "y",
            6,
        );
        let second = scatter_axes_context(
            F32Range::new(0.0, 100.0),
            F32Range::new(50.0, 150.0),
            "x",
            "y",
            6,
        );

        assert_eq!(first, second);
        assert_eq!(first.x.ticks.first().map(|tick| tick.fraction), Some(0.0));
        assert_eq!(first.x.ticks.last().map(|tick| tick.fraction), Some(1.0));
    }

    #[test]
    fn timeline_axis_ticks_include_endpoints() {
        let context = timeline_axes_context(U64Range::new(1_000, 2_000), 4, &[], 5, 4);

        let first_tick = context
            .time
            .ticks
            .first()
            .expect("first time tick must exist");
        let last_tick = context
            .time
            .ticks
            .last()
            .expect("last time tick must exist");

        assert_eq!(first_tick.label, "1000");
        assert_eq!(last_tick.label, "2000");
        assert_eq!(first_tick.fraction, 0.0);
        assert_eq!(last_tick.fraction, 1.0);
    }

    #[test]
    fn timeline_lane_labels_use_dataset_labels() {
        let context = timeline_axes_context(
            U64Range::new(1_000, 2_000),
            8,
            &[
                "blue".to_string(),
                "green".to_string(),
                "red".to_string(),
                "yellow".to_string(),
                "teal".to_string(),
                "orange".to_string(),
                "purple".to_string(),
                "black".to_string(),
            ],
            5,
            4,
        );

        assert_eq!(
            context
                .lanes
                .iter()
                .map(|lane| lane.label.clone())
                .collect::<Vec<_>>(),
            ["blue", "red", "teal", "black"]
        );
    }

    #[test]
    fn timeline_lane_labels_fall_back_to_lane_index() {
        let context =
            timeline_axes_context(U64Range::new(1_000, 2_000), 4, &["blue".to_string()], 5, 4);

        assert_eq!(
            context
                .lanes
                .iter()
                .map(|lane| lane.label.clone())
                .collect::<Vec<_>>(),
            ["blue", "lane-1", "lane-2", "lane-3"]
        );
    }
}
