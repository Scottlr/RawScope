//! Checked timestamp transforms and typed brush ranges for time-value fields.
//!
//! Timestamp microseconds are analytical source values.  This module supplies
//! the only conversion used to put them into bounded visual coordinates; it
//! deliberately does not parse or format timestamps and does not know about
//! the discrete-lane timeline view.

use std::{error::Error, fmt};

/// A checked affine transform from timestamp microseconds into `[0, 1]`.
///
/// `origin_micros` and `span_micros` retain the exact source-domain contract.
/// Differences are formed in `i128`, so domains crossing the signed integer
/// boundary cannot overflow before they are converted to the unsigned span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeAxisTransform {
    pub origin_micros: i64,
    pub span_micros: u64,
}

impl TimeAxisTransform {
    /// Builds a transform for an inclusive timestamp domain.
    ///
    /// A singleton domain is valid and maps every value to the visual centre
    /// (`0.5`).  Reversed domains are rejected before any subtraction.
    pub fn from_domain(min_micros: i64, max_micros: i64) -> Result<Self, TimeValueProjectionError> {
        if max_micros < min_micros {
            return Err(TimeValueProjectionError::ReversedDomain {
                min_micros,
                max_micros,
            });
        }

        let span_i128 = i128::from(max_micros) - i128::from(min_micros);
        let span_micros =
            u64::try_from(span_i128).map_err(|_| TimeValueProjectionError::SpanOverflow {
                min_micros,
                max_micros,
            })?;
        Ok(Self {
            origin_micros: min_micros,
            span_micros,
        })
    }

    /// Normalizes an exact timestamp to a finite visual coordinate.
    ///
    /// Values outside the source domain are clamped.  This keeps a stale or
    /// partially intersected brush from producing an unbounded GPU coordinate;
    /// membership still uses the exact typed source value elsewhere.
    pub fn normalized(self, value_micros: i64) -> f64 {
        if self.span_micros == 0 {
            return 0.5;
        }
        let offset_micros = i128::from(value_micros) - i128::from(self.origin_micros);
        let normalized = (offset_micros as f64) / (self.span_micros as f64);
        normalized.clamp(0.0, 1.0)
    }

    /// Converts a visual coordinate back to the nearest exact timestamp.
    ///
    /// Non-finite and out-of-range coordinates are handled at the typed-domain
    /// boundary: non-finite values resolve to the origin and finite values are
    /// clamped before rounding.  The checked `i128` sum is narrowed only after
    /// the result is known to lie between the original `i64` endpoints.
    pub fn denormalized(self, normalized: f64) -> i64 {
        if self.span_micros == 0 || !normalized.is_finite() {
            return self.origin_micros;
        }
        let bounded = normalized.clamp(0.0, 1.0);
        let offset_micros = (bounded * self.span_micros as f64).round();
        let offset_micros = offset_micros.clamp(0.0, self.span_micros as f64) as i128;
        let value = i128::from(self.origin_micros) + offset_micros;
        value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }

    /// Converts outward-facing normalized bounds into an exact timestamp range.
    pub fn denormalized_range(
        self,
        min_normalized: f64,
        max_normalized: f64,
    ) -> Result<(i64, i64), TimeValueProjectionError> {
        if !min_normalized.is_finite() || !max_normalized.is_finite() {
            return Err(TimeValueProjectionError::NonFiniteCoordinate);
        }
        let min_coordinate = min_normalized.min(max_normalized).clamp(0.0, 1.0);
        let max_coordinate = max_normalized.max(min_normalized).clamp(0.0, 1.0);
        let min = if self.span_micros == 0 {
            self.origin_micros
        } else {
            let offset = (min_coordinate * self.span_micros as f64).floor() as i128;
            (i128::from(self.origin_micros) + offset)
                .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
        };
        let max = if self.span_micros == 0 {
            self.origin_micros
        } else {
            let offset = (max_coordinate * self.span_micros as f64).ceil() as i128;
            (i128::from(self.origin_micros) + offset)
                .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
        };
        Ok((min, max))
    }
}

/// Failure returned when a timestamp visual domain cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeValueProjectionError {
    /// The domain's upper bound precedes its lower bound.
    ReversedDomain { min_micros: i64, max_micros: i64 },
    /// Defensive error for a future wider timestamp representation.
    SpanOverflow { min_micros: i64, max_micros: i64 },
    /// A brush/viewport supplied a non-finite normalized coordinate.
    NonFiniteCoordinate,
}

impl fmt::Display for TimeValueProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReversedDomain {
                min_micros,
                max_micros,
            } => write!(
                formatter,
                "timestamp domain is reversed (min={min_micros}, max={max_micros})"
            ),
            Self::SpanOverflow {
                min_micros,
                max_micros,
            } => write!(
                formatter,
                "timestamp span does not fit in u64 (min={min_micros}, max={max_micros})"
            ),
            Self::NonFiniteCoordinate => {
                formatter.write_str("time brush coordinates must be finite")
            }
        }
    }
}

impl Error for TimeValueProjectionError {}

/// Exact source-domain range for one visual axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualAxisSelectionRange {
    I64 { min: i64, max: i64 },
    U64 { min: u64, max: u64 },
    F64(crate::inspection::F64Domain),
    TimestampMicros { min: i64, max: i64 },
}

impl VisualAxisSelectionRange {
    pub const fn timestamp_micros(min: i64, max: i64) -> Self {
        Self::TimestampMicros { min, max }
    }

    pub const fn bounds(self) -> (f64, f64) {
        match self {
            Self::I64 { min, max } => (min as f64, max as f64),
            Self::U64 { min, max } => (min as f64, max as f64),
            Self::F64(domain) => (domain.min(), domain.max()),
            Self::TimestampMicros { min, max } => (min as f64, max as f64),
        }
    }
}

/// Immutable typed ranges produced by a visual-field rectangle brush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualFieldBrushSelection {
    pub x: VisualAxisSelectionRange,
    pub y: VisualAxisSelectionRange,
}

impl VisualFieldBrushSelection {
    pub const fn new(x: VisualAxisSelectionRange, y: VisualAxisSelectionRange) -> Self {
        Self { x, y }
    }
}

/// Failure returned when a typed brush cannot be applied to a projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualFieldBrushError {
    InvalidRange,
    AxisTypeMismatch,
}

impl fmt::Display for VisualFieldBrushError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRange => "visual-field brush range is reversed",
            Self::AxisTypeMismatch => "visual-field brush axis type does not match projection",
        })
    }
}

impl Error for VisualFieldBrushError {}

#[cfg(test)]
mod tests {
    use super::{TimeAxisTransform, TimeValueProjectionError};

    #[test]
    fn transform_uses_checked_wide_difference_across_i64_bounds() {
        let transform = TimeAxisTransform::from_domain(i64::MIN, i64::MAX).unwrap();
        assert_eq!(transform.origin_micros, i64::MIN);
        assert_eq!(transform.span_micros, u64::MAX);
        assert_eq!(transform.normalized(i64::MIN), 0.0);
        assert_eq!(transform.normalized(i64::MAX), 1.0);
    }

    #[test]
    fn reversed_time_domain_is_rejected_without_subtraction() {
        assert!(matches!(
            TimeAxisTransform::from_domain(5, 4),
            Err(TimeValueProjectionError::ReversedDomain { .. })
        ));
    }

    #[test]
    fn singleton_time_domain_maps_to_center_and_round_trips() {
        let transform = TimeAxisTransform::from_domain(42, 42).unwrap();
        assert_eq!(transform.normalized(42), 0.5);
        assert_eq!(transform.denormalized(0.0), 42);
        assert_eq!(transform.denormalized(1.0), 42);
    }

    #[test]
    fn brush_bounds_round_outward_to_exact_timestamp_values() {
        let transform = TimeAxisTransform::from_domain(0, 10).unwrap();
        assert_eq!(transform.denormalized_range(0.21, 0.29).unwrap(), (2, 3));
    }
}
