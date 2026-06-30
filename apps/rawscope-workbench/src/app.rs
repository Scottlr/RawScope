//! Native workbench application handler for the scatter-density demo.

use std::{error::Error, sync::Arc, time::Duration, time::Instant};

use rawscope_data::SyntheticPointRecord;
use rawscope_data::{generate_synthetic_points, SyntheticPointConfig};
use rawscope_gpu::{ClearFrameStatus, GpuContext};
use rawscope_render::{
    ScatterBrush, ScatterDensityRenderDiagnostics, ScatterDensityRenderer,
    ScatterDensityRendererConfig, ScatterViewport, SelectedRegionSummary,
};
use tracing::{error, info};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::MouseScrollDelta,
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::Window,
};

use crate::demo::{DemoOverlayState, PointCountPreset};

const WINDOW_TITLE: &str = "RawScope Workbench";
const INITIAL_WIDTH: f64 = 1280.0;
const INITIAL_HEIGHT: f64 = 720.0;
const DEMO_SEED: u64 = 42;
const DEMO_GRID_WIDTH: u32 = 256;
const DEMO_GRID_HEIGHT: u32 = 256;
const WHEEL_ZOOM_IN_SCALE: f32 = 0.82;
const WHEEL_ZOOM_OUT_SCALE: f32 = 1.22;
const FRAME_DIAGNOSTIC_INTERVAL: u64 = 5_000;
const PAN_DIAGNOSTIC_INTERVAL_MS: u128 = 250;

/// Winit application state for the RawScope scatter-density demo.
#[derive(Default)]
pub struct WorkbenchApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) gpu: Option<GpuContext>,
    scatter_density_renderer: Option<ScatterDensityRenderer>,
    pub(crate) points: Vec<SyntheticPointRecord>,
    active_preset: PointCountPreset,
    pub(crate) viewport: Option<ScatterViewport>,
    render_diagnostics: Option<ScatterDensityRenderDiagnostics>,
    adapter_name: Option<String>,
    backend: Option<String>,
    pub(crate) cursor_position: Option<PhysicalPosition<f64>>,
    pub(crate) last_drag_position: Option<PhysicalPosition<f64>>,
    pub(crate) modifiers: ModifiersState,
    pub(crate) brush_drag_start: Option<PhysicalPosition<f64>>,
    pub(crate) active_brush: Option<ScatterBrush>,
    pub(crate) selection_summary: Option<SelectedRegionSummary>,
    last_pan_diagnostic_at: Option<Instant>,
    redraw_count: u64,
    latest_frame_cpu_duration: Duration,
}

impl WorkbenchApp {
    pub(crate) fn create_window_and_gpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        if self.window.is_some() {
            return Ok(());
        }

        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let gpu = pollster::block_on(GpuContext::new(window.clone()))?;
        let active_preset = PointCountPreset::default();
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(
            DEMO_SEED,
            active_preset.row_count,
        ));
        let viewport = ScatterViewport::new(dataset.x_range, dataset.y_range);
        let renderer_config = ScatterDensityRendererConfig::new(
            viewport.x_range(),
            viewport.y_range(),
            DEMO_GRID_WIDTH,
            DEMO_GRID_HEIGHT,
        );
        let scatter_density_renderer = ScatterDensityRenderer::new(
            gpu.device(),
            gpu.queue(),
            gpu.surface_format(),
            &dataset.points,
            renderer_config,
        )?;
        let render_diagnostics = scatter_density_renderer.diagnostics();
        log_density_diagnostics("initial", viewport, render_diagnostics);

        let diagnostics = gpu.diagnostics();
        info!(
            adapter = %diagnostics.adapter_name,
            backend = %diagnostics.backend,
            device_type = %diagnostics.device_type,
            surface_format = %diagnostics.surface_format,
            present_mode = %diagnostics.present_mode,
            alpha_mode = %diagnostics.alpha_mode,
            "RawScope WGPU adapter selected"
        );
        info!(features = %diagnostics.adapter_features, "adapter features");
        info!(limits = %diagnostics.adapter_limits, "adapter limits");
        info!(
            seed = DEMO_SEED,
            point_count = render_diagnostics.point_count,
            grid_width = render_diagnostics.grid_width,
            grid_height = render_diagnostics.grid_height,
            max_bin_count = render_diagnostics.max_bin_count,
            "RawScope synthetic scatter-density demo prepared"
        );
        let adapter_name = diagnostics.adapter_name.clone();
        let backend = diagnostics.backend.clone();

        window.request_redraw();
        self.window = Some(window);
        self.gpu = Some(gpu);
        self.active_preset = active_preset;
        self.points = dataset.points;
        self.viewport = Some(viewport);
        self.render_diagnostics = Some(render_diagnostics);
        self.adapter_name = Some(adapter_name);
        self.backend = Some(backend);
        self.scatter_density_renderer = Some(scatter_density_renderer);
        self.update_window_title();

        Ok(())
    }

    pub(crate) fn render(&mut self, event_loop: &ActiveEventLoop) {
        let frame_start = Instant::now();
        let render_status = {
            let Some(gpu) = self.gpu.as_mut() else {
                return;
            };
            let Some(scatter_density_renderer) = self.scatter_density_renderer.as_ref() else {
                return;
            };

            gpu.render_frame(|_device, _queue, target_view, encoder| {
                scatter_density_renderer.render(encoder, target_view);
            })
        };

        match render_status {
            Ok(ClearFrameStatus::Presented) => {
                self.redraw_count += 1;
                self.latest_frame_cpu_duration = frame_start.elapsed();
                let should_log_frame = self.redraw_count.is_multiple_of(FRAME_DIAGNOSTIC_INTERVAL);
                if should_log_frame {
                    info!(
                        redraw_count = self.redraw_count,
                        frame_cpu_ms = frame_start.elapsed().as_secs_f64() * 1000.0,
                        "RawScope frame diagnostics"
                    );
                }
                self.update_window_title();
            }
            Ok(ClearFrameStatus::SkippedZeroSizedSurface | ClearFrameStatus::SkippedOccluded) => {}
            Ok(ClearFrameStatus::SkippedTimeout | ClearFrameStatus::Reconfigured) => {
                self.request_redraw();
            }
            Err(err) => {
                error!(error = %err, "failed to render clear frame");
                event_loop.exit();
            }
        }
    }

    pub(crate) fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn switch_point_preset(&mut self, preset: PointCountPreset) {
        let preset_is_already_active = self.active_preset == preset;
        if preset_is_already_active {
            return;
        }

        let dataset =
            generate_synthetic_points(SyntheticPointConfig::new(DEMO_SEED, preset.row_count));
        let viewport = ScatterViewport::new(dataset.x_range, dataset.y_range);
        self.points = dataset.points;
        self.active_preset = preset;
        self.viewport = Some(viewport);
        self.clear_brush();

        if let Err(err) = self.recompute_density("preset") {
            error!(
                error = %err,
                point_count = preset.row_count,
                "failed to recompute scatter density after preset change"
            );
        }
    }

    pub(crate) fn zoom_at_cursor(&mut self, scroll_delta: MouseScrollDelta) {
        let cursor_fraction = self.cursor_fraction();
        let Some(viewport) = self.viewport.as_mut() else {
            return;
        };

        let zoom_scroll = match scroll_delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => position.y as f32,
        };
        if zoom_scroll == 0.0 {
            return;
        }

        let zoom_scale = if zoom_scroll > 0.0 {
            WHEEL_ZOOM_IN_SCALE
        } else {
            WHEEL_ZOOM_OUT_SCALE
        };
        let (anchor_x, anchor_y) = cursor_fraction
            .map(|(x_fraction, y_fraction)| viewport.data_point_at_fraction(x_fraction, y_fraction))
            .unwrap_or_else(|| {
                let centre_fraction = 0.5;
                viewport.data_point_at_fraction(centre_fraction, centre_fraction)
            });

        viewport.zoom_around(anchor_x, anchor_y, zoom_scale);
        if let Err(err) = self.recompute_density("zoom") {
            error!(error = %err, "failed to recompute scatter density after zoom");
        }
    }

    pub(crate) fn begin_pan(&mut self) {
        self.last_drag_position = self.cursor_position;
    }

    pub(crate) fn end_pan(&mut self) {
        self.last_drag_position = None;
    }

    pub(crate) fn pan_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        let Some(last_drag_position) = self.last_drag_position else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.viewport.as_mut() else {
            return;
        };

        let window_size = window.inner_size();
        let window_has_area = window_size.width > 0 && window_size.height > 0;
        if !window_has_area {
            return;
        }

        let delta_x_fraction =
            (position.x - last_drag_position.x) as f32 / window_size.width as f32;
        let delta_y_fraction =
            (position.y - last_drag_position.y) as f32 / window_size.height as f32;
        let data_delta_x = -delta_x_fraction * viewport.x_range().span();
        let data_delta_y = delta_y_fraction * viewport.y_range().span();

        viewport.pan_by(data_delta_x, data_delta_y);
        self.last_drag_position = Some(position);
        if let Err(err) = self.recompute_density("pan") {
            error!(error = %err, "failed to recompute scatter density after pan");
        }
    }

    pub(crate) fn reset_viewport(&mut self) {
        let Some(viewport) = self.viewport.as_mut() else {
            return;
        };

        viewport.reset();
        self.clear_brush();
        if let Err(err) = self.recompute_density("reset") {
            error!(error = %err, "failed to recompute scatter density after reset");
        }
    }

    fn recompute_density(&mut self, reason: &'static str) -> Result<(), Box<dyn Error>> {
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.viewport else {
            return Ok(());
        };
        let Some(scatter_density_renderer) = self.scatter_density_renderer.as_mut() else {
            return Ok(());
        };

        let renderer_config = ScatterDensityRendererConfig::new(
            viewport.x_range(),
            viewport.y_range(),
            DEMO_GRID_WIDTH,
            DEMO_GRID_HEIGHT,
        );
        let diagnostics = scatter_density_renderer.update_density(
            gpu.device(),
            gpu.queue(),
            &self.points,
            renderer_config,
        )?;
        if self.should_log_density_update(reason) {
            log_density_diagnostics(reason, viewport, diagnostics);
        }
        self.render_diagnostics = Some(diagnostics);
        self.update_window_title();
        self.request_redraw();

        Ok(())
    }

    fn should_log_density_update(&mut self, reason: &'static str) -> bool {
        if reason != "pan" {
            return true;
        }

        let now = Instant::now();
        let should_log = self
            .last_pan_diagnostic_at
            .map(|last_log_at| {
                let elapsed_since_last_log_ms = now.duration_since(last_log_at).as_millis();
                elapsed_since_last_log_ms >= PAN_DIAGNOSTIC_INTERVAL_MS
            })
            .unwrap_or(true);
        if should_log {
            self.last_pan_diagnostic_at = Some(now);
        }

        should_log
    }

    pub(crate) fn update_window_title(&self) {
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.viewport else {
            return;
        };
        let Some(render_diagnostics) = self.render_diagnostics else {
            return;
        };
        let Some(adapter_name) = &self.adapter_name else {
            return;
        };
        let Some(backend) = &self.backend else {
            return;
        };

        let overlay = DemoOverlayState {
            preset: self.active_preset,
            viewport,
            render_diagnostics,
            selection_summary: self.selection_summary,
            redraw_count: self.redraw_count,
            latest_frame_cpu_duration: self.latest_frame_cpu_duration,
            adapter_name: adapter_name.clone(),
            backend: backend.clone(),
        };
        window.set_title(&overlay.title());
    }

    pub(crate) fn cursor_fraction(&self) -> Option<(f32, f32)> {
        let window = self.window.as_ref()?;
        let cursor_position = self.cursor_position?;
        let window_size = window.inner_size();

        let window_has_area = window_size.width > 0 && window_size.height > 0;
        if !window_has_area {
            return None;
        }

        let x_fraction = (cursor_position.x / window_size.width as f64) as f32;
        let y_fraction = (cursor_position.y / window_size.height as f64) as f32;
        Some((x_fraction.clamp(0.0, 1.0), y_fraction.clamp(0.0, 1.0)))
    }
}

fn log_density_diagnostics(
    reason: &'static str,
    viewport: ScatterViewport,
    diagnostics: ScatterDensityRenderDiagnostics,
) {
    info!(
        reason,
        point_count = diagnostics.point_count,
        grid_width = diagnostics.grid_width,
        grid_height = diagnostics.grid_height,
        x_min = viewport.x_range().min,
        x_max = viewport.x_range().max,
        y_min = viewport.y_range().min,
        y_max = viewport.y_range().max,
        max_bin_count = diagnostics.max_bin_count,
        density_update_cpu_ms = diagnostics.density_update_cpu_duration.as_secs_f64() * 1000.0,
        "RawScope scatter-density viewport updated"
    );
}
