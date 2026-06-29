//! Shared stable row-id types.

/// Identifies one row in a dataset or derived density output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct RowId(pub u64);
