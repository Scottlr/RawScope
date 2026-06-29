//! WGPU device/session and surface bootstrap for RawScope.

mod compute;
mod context;
mod diagnostics;
mod error;

pub use compute::ComputeContext;
pub use context::{ClearFrameStatus, GpuContext, DEFAULT_CLEAR_COLOR};
pub use diagnostics::{ComputeDiagnostics, GpuDiagnostics};
pub use error::GpuError;

/// Describes the role of this crate in the current scaffold.
pub fn crate_purpose() -> &'static str {
    "wgpu device/session, compute bootstrap, surface configuration, diagnostics, and clear-frame rendering"
}
