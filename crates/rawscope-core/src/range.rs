//! Checked numeric extents and display domains used by analysis and rendering.

use std::{error::Error, fmt, ops::Deref};

/// Failure returned when a numeric range cannot satisfy its domain contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RangeError {
    /// One or both floating-point bounds are not finite.
    NonFinite { min: f64, max: f64 },
    /// The lower bound is greater than the upper bound.
    Reversed { min: f64, max: f64 },
    /// A display domain requires a positive span.
    EmptyDomain { value: f64 },
    /// A singleton at the unsigned boundary cannot be expanded safely.
    CannotExpandSingleton { value: u64 },
}

impl fmt::Display for RangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { min, max } => {
                write!(
                    formatter,
                    "range bounds must be finite (min={min}, max={max})"
                )
            }
            Self::Reversed { min, max } => {
                write!(
                    formatter,
                    "range lower bound {min} exceeds upper bound {max}"
                )
            }
            Self::EmptyDomain { value } => {
                write!(
                    formatter,
                    "display domain must have positive span at {value}"
                )
            }
            Self::CannotExpandSingleton { value } => {
                write!(
                    formatter,
                    "cannot expand unsigned singleton at boundary {value}"
                )
            }
        }
    }
}

impl Error for RangeError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32RangeFields {
    pub min: f32,
    pub max: f32,
}

/// An observed finite extent. Equality is valid when all observed values match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservedF32Extent {
    fields: F32RangeFields,
}

impl ObservedF32Extent {
    pub fn try_new(min: f32, max: f32) -> Result<Self, RangeError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(RangeError::NonFinite {
                min: min as f64,
                max: max as f64,
            });
        }
        if min > max {
            return Err(RangeError::Reversed {
                min: min as f64,
                max: max as f64,
            });
        }
        Ok(Self {
            fields: F32RangeFields { min, max },
        })
    }

    pub fn min(self) -> f32 {
        self.fields.min
    }

    pub fn max(self) -> f32 {
        self.fields.max
    }

    pub fn is_singleton(self) -> bool {
        self.min() == self.max()
    }

    /// Expands only at the presentation boundary; the observed values remain exact.
    pub fn to_display_domain(self) -> Result<F32Domain, RangeError> {
        if self.min() < self.max() {
            return F32Domain::try_new(self.min(), self.max());
        }

        let epsilon = f32::EPSILON.max(self.min().abs() * f32::EPSILON);
        F32Domain::try_new(self.min() - epsilon, self.max() + epsilon)
    }
}

/// A finite, strictly increasing f32 display domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32Domain {
    fields: F32RangeFields,
}

/// Compatibility name retained for current render/data APIs.
pub type F32Range = F32Domain;

impl F32Domain {
    pub fn try_new(min: f32, max: f32) -> Result<Self, RangeError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(RangeError::NonFinite {
                min: min as f64,
                max: max as f64,
            });
        }
        if max <= min {
            return Err(RangeError::EmptyDomain { value: min as f64 });
        }
        Ok(Self {
            fields: F32RangeFields { min, max },
        })
    }

    /// Compatibility constructor for existing proven internal literals.
    pub fn new(min: f32, max: f32) -> Self {
        Self::try_new(min, max).expect("validated f32 display domain")
    }

    pub fn min(self) -> f32 {
        self.fields.min
    }

    pub fn max(self) -> f32 {
        self.fields.max
    }

    pub fn contains(self, value: f32) -> bool {
        value >= self.min() && value <= self.max()
    }

    pub fn span(self) -> f32 {
        self.max() - self.min()
    }

    /// Compatibility helper; new ingestion code should retain `ObservedF32Extent`.
    pub fn from_bounds_expanded(min: f32, max: f32) -> Self {
        ObservedF32Extent::try_new(min, max)
            .and_then(ObservedF32Extent::to_display_domain)
            .expect("validated observed f32 extent")
    }
}

impl Deref for F32Domain {
    type Target = F32RangeFields;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U64RangeFields {
    pub min: u64,
    pub max: u64,
}

/// An observed finite unsigned extent. Equality is valid for singleton data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedU64Extent {
    fields: U64RangeFields,
}

impl ObservedU64Extent {
    pub fn try_new(min: u64, max: u64) -> Result<Self, RangeError> {
        if min > max {
            return Err(RangeError::Reversed {
                min: min as f64,
                max: max as f64,
            });
        }
        Ok(Self {
            fields: U64RangeFields { min, max },
        })
    }

    pub fn min(self) -> u64 {
        self.fields.min
    }

    pub fn max(self) -> u64 {
        self.fields.max
    }

    pub fn is_singleton(self) -> bool {
        self.min() == self.max()
    }

    pub fn to_display_domain(self) -> Result<U64Range, RangeError> {
        if self.min() < self.max() {
            return U64Range::try_new(self.min(), self.max());
        }

        let expanded_min = self.min().checked_sub(1);
        let expanded_max = self.max().checked_add(1);
        match (expanded_min, expanded_max) {
            (Some(min), Some(max)) => U64Range::try_new(min, max),
            _ => Err(RangeError::CannotExpandSingleton { value: self.min() }),
        }
    }
}

/// A finite, strictly increasing unsigned display domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U64Range {
    fields: U64RangeFields,
}

impl U64Range {
    pub fn try_new(min: u64, max: u64) -> Result<Self, RangeError> {
        if max <= min {
            return Err(RangeError::EmptyDomain { value: min as f64 });
        }
        Ok(Self {
            fields: U64RangeFields { min, max },
        })
    }

    pub fn new(min: u64, max: u64) -> Self {
        Self::try_new(min, max).expect("validated u64 display domain")
    }

    pub fn min(self) -> u64 {
        self.fields.min
    }

    pub fn max(self) -> u64 {
        self.fields.max
    }

    pub fn contains(self, value: u64) -> bool {
        value >= self.min() && value <= self.max()
    }

    pub fn span(self) -> u64 {
        self.max() - self.min()
    }

    pub fn from_bounds_expanded(min: u64, max: u64) -> Self {
        ObservedU64Extent::try_new(min, max)
            .and_then(ObservedU64Extent::to_display_domain)
            .expect("validated observed u64 extent")
    }
}

impl Deref for U64Range {
    type Target = U64RangeFields;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_f32_singleton_expands_only_when_displayed() {
        let observed = ObservedF32Extent::try_new(12.0, 12.0).unwrap();
        assert_eq!(observed.min(), 12.0);
        assert_eq!(observed.max(), 12.0);

        let display = observed.to_display_domain().unwrap();
        assert!(display.min() < 12.0);
        assert!(display.max() > 12.0);
    }

    #[test]
    fn invalid_domains_return_errors_without_panicking() {
        assert!(matches!(
            F32Domain::try_new(f32::NAN, 1.0),
            Err(RangeError::NonFinite { .. })
        ));
        assert!(matches!(
            U64Range::try_new(2, 1),
            Err(RangeError::EmptyDomain { .. })
        ));
    }

    #[test]
    fn observed_u64_boundary_singleton_cannot_be_expanded() {
        assert!(matches!(
            ObservedU64Extent::try_new(u64::MAX, u64::MAX)
                .unwrap()
                .to_display_domain(),
            Err(RangeError::CannotExpandSingleton { value: u64::MAX })
        ));
    }
}
