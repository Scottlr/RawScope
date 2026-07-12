//! Checked memory-accounting policy for dataset store admission.

use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetMemoryBudget {
    max_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetMemoryUsage {
    retained_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetBudgetError {
    pub requested_bytes: u64,
    pub max_bytes: u64,
}

impl fmt::Display for DatasetBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "dataset memory budget exceeded: requested {} bytes, maximum {} bytes",
            self.requested_bytes, self.max_bytes
        )
    }
}

impl Error for DatasetBudgetError {}

impl DatasetMemoryBudget {
    pub const fn new(max_bytes: u64) -> Self {
        Self { max_bytes }
    }

    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }

    pub const fn admits(self, requested_bytes: u64) -> bool {
        requested_bytes <= self.max_bytes
    }
}

impl DatasetMemoryUsage {
    pub const fn new(retained_bytes: u64) -> Self {
        Self { retained_bytes }
    }

    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }
}
