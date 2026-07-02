//! WGPU device/session and surface bootstrap for RawScope.

mod adapter;
mod compute;
mod context;
mod error;

pub use adapter::{ComputeAdapterInfo, GpuAdapterInfo};
pub use compute::ComputeContext;
pub use context::{ClearFrameStatus, GpuContext};
pub use error::GpuError;
