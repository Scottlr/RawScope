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
}
