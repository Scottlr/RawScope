//! WGPU device/session and surface bootstrap for RawScope.

mod adapter;
mod compute;
mod context;
mod error;
mod limits;
mod policy;
mod readback;
mod recovery;

pub use adapter::{ComputeAdapterInfo, GpuAdapterInfo};
pub use compute::ComputeContext;
pub use context::{ClearFrameStatus, GpuContext};
pub use error::{GpuError, GpuRuntimeSignal};
pub use limits::{GpuLimitError, GpuResourcePlan};
pub use policy::{AdapterPolicy, FallbackPolicy};
pub use readback::{
    GpuReadbackTicket, ReadbackCompletion, ReadbackError, ReadbackGeneration, ReadbackProgress,
    ReadbackState,
};
pub use recovery::{DeviceGeneration, DeviceLossReason, GpuRecoveryState};
