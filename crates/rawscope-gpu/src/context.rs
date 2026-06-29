//! Minimal WGPU context for surface configuration and clear-frame presentation.

use std::sync::Arc;

use tracing::{info, warn};
use wgpu::{CurrentSurfaceTexture, SurfaceTexture};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{GpuDiagnostics, GpuError};

/// Default clear colour for the Milestone 2 bootstrap surface.
pub const DEFAULT_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.015,
    g: 0.025,
    b: 0.035,
    a: 1.0,
};

/// Result of attempting to clear one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearFrameStatus {
    Presented,
    SkippedZeroSizedSurface,
    SkippedTimeout,
    SkippedOccluded,
    Reconfigured,
}

/// Owns RawScope's initial WGPU instance, surface, device, queue, and diagnostics.
pub struct GpuContext {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    diagnostics: GpuDiagnostics,
}

impl GpuContext {
    /// Initializes WGPU for the provided native window.
    pub async fn new(window: Arc<Window>) -> Result<Self, GpuError> {
        let size = non_zero_size(window.inner_size());
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window)
            .map_err(GpuError::CreateSurface)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(GpuError::RequestAdapter)?;

        let adapter_info = adapter.get_info();
        let adapter_features = adapter.features();
        let adapter_limits = adapter.limits();

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

        let diagnostics =
            GpuDiagnostics::from_parts(adapter_info, adapter_features, adapter_limits, &config);
        info!(
            adapter = %diagnostics.adapter_name,
            backend = %diagnostics.backend,
            device_type = %diagnostics.device_type,
            format = %diagnostics.surface_format,
            present_mode = %diagnostics.present_mode,
            "initialized WGPU context"
        );

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            size,
            diagnostics,
        })
    }

    /// Returns startup diagnostics for the selected adapter and surface.
    pub fn diagnostics(&self) -> &GpuDiagnostics {
        &self.diagnostics
    }

    /// Reconfigures the surface after a window resize.
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> ClearFrameStatus {
        self.size = size;

        if size.width == 0 || size.height == 0 {
            return ClearFrameStatus::SkippedZeroSizedSurface;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        ClearFrameStatus::Reconfigured
    }

    /// Clears and presents one frame with the provided colour.
    pub fn clear_frame(&mut self, clear_color: wgpu::Color) -> Result<ClearFrameStatus, GpuError> {
        if self.size.width == 0 || self.size.height == 0 {
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

        self.clear_surface_texture(frame, clear_color);
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

    fn clear_surface_texture(&self, frame: SurfaceTexture, clear_color: wgpu::Color) {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RawScope Clear Frame Encoder"),
            });

        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RawScope Clear Frame Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

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
