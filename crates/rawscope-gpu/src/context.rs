//! Minimal WGPU context for surface configuration and clear-frame presentation.

use std::sync::Arc;

use tracing::{info, warn};
use wgpu::{CurrentSurfaceTexture, SurfaceTexture, TextureView};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{AdapterPolicy, GpuAdapterInfo, GpuError};

/// Result of attempting to present one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearFrameStatus {
    Presented,
    SkippedZeroSizedSurface,
    SkippedTimeout,
    SkippedOccluded,
    Reconfigured,
}

/// Owns RawScope's initial WGPU instance, surface, device, and queue.
pub struct GpuContext {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    adapter_info: GpuAdapterInfo,
}

impl GpuContext {
    /// Initializes WGPU for the provided native window.
    pub async fn new(window: Arc<Window>) -> Result<Self, GpuError> {
        let size = non_zero_size(window.inner_size());
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window)
            .map_err(GpuError::CreateSurface)?;

        let adapter_policy = AdapterPolicy::default();
        let adapter = match instance
            .request_adapter(&adapter_policy.request_options(Some(&surface)))
            .await
        {
            Ok(adapter) => adapter,
            Err(error) if adapter_policy.allows_fallback() => instance
                .request_adapter(&adapter_policy.fallback_request_options(Some(&surface)))
                .await
                .map_err(GpuError::RequestAdapter)
                .inspect_err(|_fallback_error| {
                    warn!(
                        ?error,
                        "preferred WGPU adapter unavailable; software fallback failed"
                    );
                })?,
            Err(error) => return Err(GpuError::RequestAdapter(error)),
        };

        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("RawScope WGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(GpuError::RequestDevice)?;

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or(GpuError::MissingSurfaceConfig)?;
        surface.configure(&device, &config);

        let adapter_info = GpuAdapterInfo::from_parts(adapter_info, &config);
        info!(
            adapter = %adapter_info.adapter_name,
            backend = %adapter_info.backend,
            device_type = %adapter_info.device_type,
            format = %adapter_info.surface_format,
            present_mode = %adapter_info.present_mode,
            "initialized WGPU context"
        );

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            size,
            adapter_info,
        })
    }

    /// Returns metadata for the selected adapter and surface.
    pub fn adapter_info(&self) -> &GpuAdapterInfo {
        &self.adapter_info
    }

    /// Returns the WGPU device selected for this window context.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the WGPU queue selected for this window context.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns the configured surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Reconfigures the surface after a window resize.
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> ClearFrameStatus {
        self.size = size;

        let surface_is_zero_sized = size.width == 0 || size.height == 0;
        if surface_is_zero_sized {
            return ClearFrameStatus::SkippedZeroSizedSurface;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        ClearFrameStatus::Reconfigured
    }

    /// Acquires, renders, submits, and presents one surface frame.
    pub fn render_frame(
        &mut self,
        render: impl FnOnce(&wgpu::Device, &wgpu::Queue, &TextureView, &mut wgpu::CommandEncoder),
    ) -> Result<ClearFrameStatus, GpuError> {
        let surface_is_zero_sized = self.size.width == 0 || self.size.height == 0;
        if surface_is_zero_sized {
            return Ok(ClearFrameStatus::SkippedZeroSizedSurface);
        }

        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Timeout => return Ok(ClearFrameStatus::SkippedTimeout),
            CurrentSurfaceTexture::Occluded => return Ok(ClearFrameStatus::SkippedOccluded),
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.reconfigure_current_size();
                return Ok(ClearFrameStatus::Reconfigured);
            }
            CurrentSurfaceTexture::Validation => return Err(GpuError::SurfaceValidation),
        };

        self.render_surface_texture(frame, render);
        Ok(ClearFrameStatus::Presented)
    }

    fn reconfigure_current_size(&self) {
        warn!(
            width = self.config.width,
            height = self.config.height,
            "reconfiguring WGPU surface"
        );
        self.surface.configure(&self.device, &self.config);
    }

    fn render_surface_texture(
        &self,
        frame: SurfaceTexture,
        render: impl FnOnce(&wgpu::Device, &wgpu::Queue, &TextureView, &mut wgpu::CommandEncoder),
    ) {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RawScope Surface Frame Encoder"),
            });

        render(&self.device, &self.queue, &view, &mut encoder);

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

fn non_zero_size(size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width.max(1), size.height.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_zero_size_clamps_zero_dimensions() {
        assert_eq!(
            non_zero_size(PhysicalSize::new(0, 480)),
            PhysicalSize::new(1, 480)
        );
        assert_eq!(
            non_zero_size(PhysicalSize::new(640, 0)),
            PhysicalSize::new(640, 1)
        );
    }
}
