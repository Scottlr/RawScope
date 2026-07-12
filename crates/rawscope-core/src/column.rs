//! Stable column identifiers and bounded row limits.

use std::{error::Error, fmt, num::NonZeroU64};

/// Identifies a stable column position in a validated table schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnId(u32);

impl ColumnId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A positive row limit that can be converted to an allocation size explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PositiveRowLimit(NonZeroU64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveRowLimitError;

impl fmt::Display for PositiveRowLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "row limit must be greater than zero")
    }
}

impl Error for PositiveRowLimitError {}

impl PositiveRowLimit {
    pub fn try_new(value: u64) -> Result<Self, PositiveRowLimitError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(PositiveRowLimitError)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }

    pub fn try_as_usize(self) -> Result<usize, PositiveRowLimitError> {
        usize::try_from(self.get()).map_err(|_| PositiveRowLimitError)
    }
}

impl TryFrom<u64> for PositiveRowLimit {
    type Error = PositiveRowLimitError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}
