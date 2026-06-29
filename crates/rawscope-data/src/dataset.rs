//! Shared metadata for synthetic datasets.

/// Metadata describing how a synthetic dataset was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticDatasetMetadata {
    pub seed: u64,
    pub row_count: usize,
}

impl SyntheticDatasetMetadata {
    /// Creates metadata for a deterministic synthetic dataset.
    pub fn new(seed: u64, row_count: usize) -> Self {
        Self { seed, row_count }
    }
}
