//! Shared foundational types for RawScope.

/// Identifies a user-driven selection in the visual exploration pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SelectionId(pub u64);

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "shared types, ranges, view specs, and selections"
}
