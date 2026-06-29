//! Future GPU session and resource management for RawScope.

/// Placeholder marker for future GPU session ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuSessionMarker;

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "future wgpu device/session, buffers, and shader loading"
}
