//! Minimal WGPU context for surface configuration and clear-frame presentation.

use std::sync::{Arc, Mutex};

use tracing::{info, warn};
use wgpu::{CurrentSurfaceTexture, SurfaceTexture, TextureView};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    AdapterPolicy, DeviceGeneration, DeviceLossReason, GpuAdapterInfo, GpuError, GpuRecoveryState,
    GpuRuntimeSignal,
};

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
    runtime_signals: Arc<Mutex<Vec<GpuRuntimeSignal>>>,
    recovery_state: GpuRecoveryState,
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
        let (adapter, used_software_fallback) = match instance
            .request_adapter(&adapter_policy.request_options(Some(&surface)))
            .await
        {
            Ok(adapter) => (adapter, false),
            Err(error) if adapter_policy.allows_fallback() => instance
                .request_adapter(&adapter_policy.fallback_request_options(Some(&surface)))
                .await
                .map_err(GpuError::RequestAdapter)
                .inspect_err(|_fallback_error| {
                    warn!(
                        ?error,
                        "preferred WGPU adapter unavailable; software fallback failed"
                    );
                })
                .map(|adapter| (adapter, true))?,
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

        let runtime_signals = Arc::new(Mutex::new(Vec::new()));
        let uncaptured_signals = Arc::clone(&runtime_signals);
        device.on_uncaptured_error(Arc::new(move |error| {
            if let Ok(mut signals) = uncaptured_signals.lock() {
                signals.push(GpuRuntimeSignal::UncapturedError {
                    message: error.to_string(),
                });
            }
        }));
        let lost_signals = Arc::clone(&runtime_signals);
        device.set_device_lost_callback(move |reason, message| {
            if let Ok(mut signals) = lost_signals.lock() {
                signals.push(GpuRuntimeSignal::DeviceLost {
                    reason: format!("{reason:?}"),
                    message,
                });
            }
        });

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or(GpuError::MissingSurfaceConfig)?;
        let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        surface.configure(&device, &config);
        if let Some(source) = validation_scope.pop().await {
            return Err(GpuError::ValidationScope {
                operation: "initial surface configuration",
                source,
            });
        }

        let adapter_info =
            GpuAdapterInfo::from_parts(adapter_info, &config, used_software_fallback);
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
            runtime_signals,
            recovery_state: GpuRecoveryState::Ready(DeviceGeneration(0)),
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

    /// Drains uncaptured validation/device-loss signals observed by WGPU.
    pub fn drain_runtime_signals(&mut self) -> Vec<GpuRuntimeSignal> {
        let signals = self
            .runtime_signals
            .lock()
            .map(|mut signals| std::mem::take(&mut *signals))
            .unwrap_or_default();
        for signal in &signals {
            if matches!(signal, GpuRuntimeSignal::DeviceLost { .. }) {
                self.recovery_state = self.recovery_state.device_lost(DeviceLossReason::Unknown);
            }
        }
        signals
    }

    /// Returns the current device/surface recovery state.
    pub fn recovery_state(&self) -> GpuRecoveryState {
        self.recovery_state
    }

    /// Marks a complete replacement context as ready.
    pub fn complete_recovery(&mut self) -> bool {
        let next = self.recovery_state.recovered();
        let changed = next != self.recovery_state;
        self.recovery_state = next;
        changed
    }

    /// Begins rebuilding after a reported device loss.
    pub fn begin_recovery(&mut self) -> bool {
        let next = self.recovery_state.begin_recovery();
        let changed = next != self.recovery_state;
        self.recovery_state = next;
        changed
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
            CurrentSurfaceTexture::Success(frame) => frame,
            CurrentSurfaceTexture::Suboptimal(frame) => {
                self.recovery_state = self.recovery_state.surface_outdated();
                self.reconfigure_current_size();
                self.recovery_state = self.recovery_state.surface_reconfigured();
                frame
            }
            CurrentSurfaceTexture::Timeout => return Ok(ClearFrameStatus::SkippedTimeout),
            CurrentSurfaceTexture::Occluded => return Ok(ClearFrameStatus::SkippedOccluded),
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.recovery_state = self.recovery_state.surface_outdated();
                self.reconfigure_current_size();
                return Ok(ClearFrameStatus::Reconfigured);
            }
            CurrentSurfaceTexture::Validation => {
                self.recovery_state = GpuRecoveryState::Fatal;
                return Err(GpuError::SurfaceValidation);
            }
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
