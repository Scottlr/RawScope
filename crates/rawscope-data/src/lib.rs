//! Future dataset metadata and columnar abstractions for RawScope.

/// Minimal dataset summary used to mark the crate's intended ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetSummary {
    pub name: String,
    pub row_count: u64,
}

impl DatasetSummary {
    /// Creates a placeholder dataset summary for scaffold-time wiring.
    pub fn new(name: impl Into<String>, row_count: u64) -> Self {
        Self {
            name: name.into(),
            row_count,
        }
    }
}

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "future columnar data abstractions, dataset metadata, and row ids"
}
