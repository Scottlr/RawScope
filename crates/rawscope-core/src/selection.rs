//! Shared selection identifiers for future interaction flows.

/// Identifies a user-driven selection in the visual exploration pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SelectionId(pub u64);
