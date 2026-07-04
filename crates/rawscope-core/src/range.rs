//! Shared numeric range types used by synthetic data and density binning.

/// A floating-point range with inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32Range {
    pub min: f32,
    pub max: f32,
}

impl F32Range {
    /// Creates a new floating-point range.
    pub fn new(min: f32, max: f32) -> Self {
        assert!(max > min, "range max must be greater than min");
        Self { min, max }
    }

    /// Returns true when the value lies inside the range.
    pub fn contains(self, value: f32) -> bool {
        value >= self.min && value <= self.max
    }

    /// Returns the numeric span.
    pub fn span(self) -> f32 {
        self.max - self.min
    }

    /// Creates a range from observed bounds, expanding a single-value extent.
    pub fn from_bounds_expanded(min: f32, max: f32) -> Self {
        if max > min {
            return Self::new(min, max);
        }

        let epsilon = f32::EPSILON.max(min.abs() * f32::EPSILON);
        Self::new(min - epsilon, max + epsilon)
    }
}

/// An integer range with inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U64Range {
    pub min: u64,
    pub max: u64,
}

impl U64Range {
    /// Creates a new integer range.
    pub fn new(min: u64, max: u64) -> Self {
        assert!(max > min, "range max must be greater than min");
        Self { min, max }
    }

    /// Returns true when the value lies inside the range.
    pub fn contains(self, value: u64) -> bool {
        value >= self.min && value <= self.max
    }

    /// Returns the numeric span.
    pub fn span(self) -> u64 {
        self.max - self.min
    }

    /// Creates a range from observed bounds, expanding a single-value extent.
    pub fn from_bounds_expanded(min: u64, max: u64) -> Self {
        if max > min {
            return Self::new(min, max);
        }

        let expanded_min = min.saturating_sub(1);
        let expanded_max = max.saturating_add(1);
        if expanded_max > expanded_min {
            return Self::new(expanded_min, expanded_max);
        }

        Self::new(min - 1, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_range_from_bounds_expands_single_value_extent() {
        let range = F32Range::from_bounds_expanded(12.0, 12.0);

        assert!(range.min < 12.0);
        assert!(range.max > 12.0);
    }

    #[test]
    fn u64_range_from_bounds_expands_single_value_extent() {
        let range = U64Range::from_bounds_expanded(12, 12);

        assert_eq!(range, U64Range::new(11, 13));
    }
}
