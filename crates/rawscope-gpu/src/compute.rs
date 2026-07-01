//! Headless WGPU device/session bootstrap for correctness compute passes.

use tracing::info;

use crate::{ComputeAdapterInfo, GpuError};

/// Owns a WGPU instance, device, and queue without a presentation surface.
pub struct ComputeContext {
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: ComputeAdapterInfo,
}

impl ComputeContext {
    /// Initializes WGPU for headless compute work.
    pub async fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(GpuError::RequestAdapter)?;

        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("RawScope WGPU Compute Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(GpuError::RequestDevice)?;

        let adapter_info = ComputeAdapterInfo::from_parts(adapter_info);
        info!(
            adapter = %adapter_info.adapter_name,
            backend = %adapter_info.backend,
            device_type = %adapter_info.device_type,
            "initialized headless WGPU compute context"
        );

        Ok(Self {
            _instance: instance,
            device,
            queue,
            adapter_info,
        })
    }

    /// Returns the selected WGPU device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the selected WGPU queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns metadata for the selected compute adapter.
    pub fn adapter_info(&self) -> &ComputeAdapterInfo {
        &self.adapter_info
    }
}
