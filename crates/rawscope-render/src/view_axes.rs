//! Axis and lane context for density views.

use rawscope_core::{F32Range, U64Range};

const MIN_AXIS_TICK_COUNT: usize = 2;
const MAX_AXIS_TICKS: usize = 9;

/// Explicit numeric label formatting for an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisValueFormat {
    Integer,
    Decimal { max_fraction_digits: u8 },
    Compact,
}

/// Data-space equation represented by a scatter reference guide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScatterReferenceGuideKind {
    Equality,
    Horizontal { y: f32 },
    Vertical { x: f32 },
}

/// Requested scatter reference guide.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterReferenceGuide {
    pub kind: ScatterReferenceGuideKind,
    pub label: String,
}

/// Plot-fraction segment produced by clipping a guide to visible data ranges.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterReferenceGuideSegment {
    pub kind: ScatterReferenceGuideKind,
    pub label: String,
    pub start_fraction: (f32, f32),
    pub end_fraction: (f32, f32),
}

/// Bounded options for scatter axes without introducing a style DSL.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterAxesOptions {
    pub target_tick_count: usize,
    pub x_format: AxisValueFormat,
    pub y_format: AxisValueFormat,
    pub guides: Vec<ScatterReferenceGuide>,
}

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
    pub guides: Vec<ScatterReferenceGuideSegment>,
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
    scatter_axes_context_with_options(
        x_range,
        y_range,
        x_label,
        y_label,
        ScatterAxesOptions {
            target_tick_count: max_ticks,
            x_format: AxisValueFormat::Decimal {
                max_fraction_digits: 1,
            },
            y_format: AxisValueFormat::Decimal {
                max_fraction_digits: 1,
            },
            guides: Vec::new(),
        },
    )
}

/// Builds scatter axes with explicit formatting and reference guides.
pub fn scatter_axes_context_with_options(
    x_range: F32Range,
    y_range: F32Range,
    x_label: impl Into<String>,
    y_label: impl Into<String>,
    options: ScatterAxesOptions,
) -> ScatterAxesContext {
    let tick_count = clamp_tick_count(options.target_tick_count).min(MAX_AXIS_TICKS);

    ScatterAxesContext {
        x: NumericAxisContext {
            label: x_label.into(),
            ticks: nice_numeric_axis_ticks(x_range, tick_count, options.x_format),
        },
        y: NumericAxisContext {
            label: y_label.into(),
            ticks: nice_numeric_axis_ticks(y_range, tick_count, options.y_format),
        },
        guides: options
            .guides
            .into_iter()
            .filter_map(|guide| project_guide(guide, x_range, y_range))
            .collect(),
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

fn nice_numeric_axis_ticks(
    range: F32Range,
    target_tick_count: usize,
    format: AxisValueFormat,
) -> Vec<AxisTick> {
    let span = range.span();
    if !span.is_finite() || span <= 0.0 || target_tick_count < 2 {
        return Vec::new();
    }

    let raw_step = span / (target_tick_count.saturating_sub(1)) as f32;
    let magnitude = 10.0_f32.powf(raw_step.abs().log10().floor());
    let normalized = raw_step / magnitude;
    let step_factor = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = step_factor * magnitude;
    let first = (range.min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut value = first;
    while value <= range.max + step * 0.0001 && ticks.len() < MAX_AXIS_TICKS {
        let fraction = ((value - range.min) / span).clamp(0.0, 1.0);
        let label = format_axis_value(value, format);
        if ticks
            .last()
            .is_none_or(|tick: &AxisTick| tick.label != label)
        {
            ticks.push(AxisTick { fraction, label });
        }
        value += step;
    }
    ticks
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

pub fn format_axis_value(value: f32, format: AxisValueFormat) -> String {
    match format {
        AxisValueFormat::Integer => format!("{value:.0}"),
        AxisValueFormat::Decimal {
            max_fraction_digits,
        } => {
            let digits = usize::from(max_fraction_digits.min(6));
            let formatted = format!("{value:.digits$}");
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
        AxisValueFormat::Compact if value.abs() >= 1_000_000.0 => {
            format!("{:.1}M", value / 1_000_000.0)
        }
        AxisValueFormat::Compact if value.abs() >= 1_000.0 => {
            format!("{:.1}k", value / 1_000.0)
        }
        AxisValueFormat::Compact => format_axis_value(
            value,
            AxisValueFormat::Decimal {
                max_fraction_digits: 1,
            },
        ),
    }
}

fn project_guide(
    guide: ScatterReferenceGuide,
    x_range: F32Range,
    y_range: F32Range,
) -> Option<ScatterReferenceGuideSegment> {
    let (start, end) = match guide.kind {
        ScatterReferenceGuideKind::Equality => {
            let min = x_range.min.max(y_range.min);
            let max = x_range.max.min(y_range.max);
            (max > min).then_some(((min, min), (max, max)))?
        }
        ScatterReferenceGuideKind::Horizontal { y } if y_range.contains(y) => {
            ((x_range.min, y), (x_range.max, y))
        }
        ScatterReferenceGuideKind::Vertical { x } if x_range.contains(x) => {
            ((x, y_range.min), (x, y_range.max))
        }
        _ => return None,
    };
    let project = |(x, y): (f32, f32)| {
        (
            ((x - x_range.min) / x_range.span()).clamp(0.0, 1.0),
            ((y_range.max - y) / y_range.span()).clamp(0.0, 1.0),
        )
    };
    Some(ScatterReferenceGuideSegment {
        kind: guide.kind,
        label: guide.label,
        start_fraction: project(start),
        end_fraction: project(end),
    })
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
