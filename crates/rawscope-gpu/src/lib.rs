//! WGPU device/session and surface bootstrap for RawScope.

mod context;
mod diagnostics;
mod error;

pub use context::{ClearFrameStatus, GpuContext, DEFAULT_CLEAR_COLOR};
pub use diagnostics::GpuDiagnostics;
pub use error::GpuError;

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "wgpu device/session, surface configuration, diagnostics, and clear-frame rendering"
}
