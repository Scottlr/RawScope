//! Normative bounded scalar-axis tick math shared by render consumers.

use rawscope_core::F32Range;

const MIN_AXIS_TICK_COUNT: usize = 2;
const MAX_AXIS_TICKS: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisValueFormat {
    Integer,
    Decimal { max_fraction_digits: u8 },
    Compact,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisTick {
    pub fraction: f32,
    pub label: String,
}

pub fn clamp_tick_count(requested: usize) -> usize {
    requested.clamp(MIN_AXIS_TICK_COUNT, MAX_AXIS_TICKS)
}

pub fn numeric_axis_ticks(
    range: F32Range,
    target_tick_count: usize,
    format: AxisValueFormat,
) -> Vec<AxisTick> {
    let target_tick_count = clamp_tick_count(target_tick_count);
    let span = range.span();
    if !span.is_finite() || span <= 0.0 {
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
    for tick_index in 0..MAX_AXIS_TICKS {
        let value = first + (tick_index as f32 * step);
        if !value.is_finite() || value > range.max + step * 0.0001 {
            break;
        }
        let fraction = ((value - range.min) / span).clamp(0.0, 1.0);
        if ticks
            .last()
            .is_some_and(|tick: &AxisTick| tick.fraction >= fraction)
        {
            break;
        }
        let label = format_axis_value(value, format);
        if ticks
            .last()
            .is_none_or(|tick: &AxisTick| tick.label != label)
        {
            ticks.push(AxisTick { fraction, label });
        }
    }
    ticks
}

pub fn u64_axis_ticks(min: u64, max: u64, tick_count: usize) -> Vec<AxisTick> {
    let tick_count = clamp_tick_count(tick_count);
    let span = u128::from(max).saturating_sub(u128::from(min));
    let last_index = tick_count - 1;
    (0..tick_count)
        .map(|tick_index| {
            let fraction = tick_index as f64 / last_index as f64;
            let numerator = span * u128::from(tick_index as u64);
            let denominator = u128::from(last_index as u64);
            let offset = (numerator + denominator / 2) / denominator;
            let value = u64::try_from(u128::from(min) + offset).unwrap_or(max);
            AxisTick {
                fraction: (fraction as f32).clamp(0.0, 1.0),
                label: value.to_string(),
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
            format!("{value:.digits$}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_tick_count_is_bounded_and_never_empty_for_valid_range() {
        let ticks = numeric_axis_ticks(
            F32Range::new(-1.0, 1.0),
            usize::MAX,
            AxisValueFormat::Decimal {
                max_fraction_digits: 2,
            },
        );
        assert!((2..=MAX_AXIS_TICKS).contains(&ticks.len()));
    }

    #[test]
    fn u64_ticks_preserve_maximum_span_without_overflow() {
        let ticks = u64_axis_ticks(0, u64::MAX, 3);
        assert_eq!(ticks.first().unwrap().label, "0");
        assert_eq!(ticks.last().unwrap().label, u64::MAX.to_string());
    }
}
