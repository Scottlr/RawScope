//! WGPU device/session and surface bootstrap for RawScope.

mod adapter;
mod compute;
mod context;
mod error;
mod limits;

pub use adapter::{ComputeAdapterInfo, GpuAdapterInfo};
pub use compute::ComputeContext;
pub use context::{ClearFrameStatus, GpuContext};
pub use error::GpuError;
pub use limits::{GpuLimitError, GpuResourcePlan};
