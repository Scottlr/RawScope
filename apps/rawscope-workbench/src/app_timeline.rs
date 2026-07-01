//! Timeline-density demo setup and time-axis interaction.

use std::{error::Error, time::Instant};

use rawscope_data::{generate_synthetic_events, SyntheticEventConfig};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    TimelineDensityRenderDiagnostics, TimelineDensityRenderer, TimelineDensityRendererConfig,
    TimelineViewport,
};
use tracing::{error, info};
use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

use crate::app::{
    WorkbenchApp, DEMO_SEED, PAN_DIAGNOSTIC_INTERVAL_MS, WHEEL_ZOOM_IN_SCALE,
    WHEEL_ZOOM_OUT_SCALE,
};

const TIMELINE_DEMO_EVENT_COUNT: usize = 20_000;
const TIMELINE_DEMO_GRID_WIDTH: u32 = 256;

impl WorkbenchApp {
    pub(crate) fn prepare_timeline_demo(&mut self, gpu: &GpuContext) -> Result<(), Box<dyn Error>> {
        let dataset = generate_synthetic_events(SyntheticEventConfig::new(
            DEMO_SEED,
            TIMELINE_DEMO_EVENT_COUNT,
        ));
        let viewport = TimelineViewport::new(dataset.time_range, dataset.lane_count);
        let renderer_config = timeline_renderer_config(viewport);
        let timeline_density_renderer = TimelineDensityRenderer::new(
            gpu.device(),
            gpu.queue(),
            gpu.surface_format(),
            &dataset.events,
            renderer_config,
        )?;
        let render_diagnostics = timeline_density_renderer.diagnostics();
        log_timeline_density_diagnostics("initial", viewport, render_diagnostics);

        self.dataset_metadata = Some(dataset.metadata);
        self.events = dataset.events;
        self.timeline_viewport = Some(viewport);
        self.timeline_render_diagnostics = Some(render_diagnostics);
        self.timeline_density_renderer = Some(timeline_density_renderer);

        Ok(())
    }

    pub(crate) fn zoom_timeline_at_cursor(&mut self, scroll_delta: MouseScrollDelta) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let cursor_x_fraction = self
            .cursor_fraction()
            .map(|(x_fraction, _y_fraction)| x_fraction)
            .unwrap_or(0.5);
        let Some(viewport) = self.timeline_viewport.as_mut() else {
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

        viewport.zoom_around_fraction(cursor_x_fraction, zoom_scale);
        if let Err(err) = self.recompute_timeline_density("zoom") {
            error!(error = %err, "failed to recompute timeline density after zoom");
        }
    }

    pub(crate) fn begin_timeline_pan(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        self.last_drag_position = self.cursor_position;
    }

    pub(crate) fn pan_timeline_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let Some(last_drag_position) = self.last_drag_position else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.timeline_viewport.as_mut() else {
            return;
        };

        let window_size = window.inner_size();
        let window_has_width = window_size.width > 0;
        if !window_has_width {
            return;
        }

        let delta_x_fraction =
            (position.x - last_drag_position.x) as f64 / window_size.width as f64;
        let current_time_span = viewport.time_range().span() as f64;
        let time_delta = (-delta_x_fraction * current_time_span).round() as i64;

        viewport.pan_by(time_delta);
        self.last_drag_position = Some(position);
        if let Err(err) = self.recompute_timeline_density("pan") {
            error!(error = %err, "failed to recompute timeline density after pan");
        }
    }

    pub(crate) fn reset_timeline_viewport(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let Some(viewport) = self.timeline_viewport.as_mut() else {
            return;
        };

        viewport.reset();
        if let Err(err) = self.recompute_timeline_density("reset") {
            error!(error = %err, "failed to recompute timeline density after reset");
        }
    }

    fn recompute_timeline_density(&mut self, reason: &'static str) -> Result<(), Box<dyn Error>> {
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.timeline_viewport else {
            return Ok(());
        };
        let Some(timeline_density_renderer) = self.timeline_density_renderer.as_mut() else {
            return Ok(());
        };

        let renderer_config = timeline_renderer_config(viewport);
        let diagnostics = timeline_density_renderer.update_density(
            gpu.device(),
            gpu.queue(),
            &self.events,
            renderer_config,
        )?;
        if self.should_log_timeline_density_update(reason) {
            log_timeline_density_diagnostics(reason, viewport, diagnostics);
        }
        self.timeline_render_diagnostics = Some(diagnostics);
        self.update_window_title();
        self.request_redraw();

        Ok(())
    }

    fn should_log_timeline_density_update(&mut self, reason: &'static str) -> bool {
        if reason != "pan" {
            return true;
        }

        let now = Instant::now();
        let should_log = self
            .last_timeline_pan_diagnostic_at
            .map(|last_log_at| {
                let elapsed_since_last_log_ms = now.duration_since(last_log_at).as_millis();
                elapsed_since_last_log_ms >= PAN_DIAGNOSTIC_INTERVAL_MS
            })
            .unwrap_or(true);
        if should_log {
            self.last_timeline_pan_diagnostic_at = Some(now);
        }

        should_log
    }
}

fn timeline_renderer_config(viewport: TimelineViewport) -> TimelineDensityRendererConfig {
    TimelineDensityRendererConfig::new(
        viewport.time_range(),
        viewport.lane_count(),
        TIMELINE_DEMO_GRID_WIDTH,
        viewport.lane_count(),
    )
}

fn log_timeline_density_diagnostics(
    reason: &'static str,
    viewport: TimelineViewport,
    diagnostics: TimelineDensityRenderDiagnostics,
) {
    info!(
        reason,
        event_count = diagnostics.event_count,
        lane_count = diagnostics.lane_count,
        grid_width = diagnostics.grid_width,
        grid_height = diagnostics.grid_height,
        time_min = viewport.time_range().min,
        time_max = viewport.time_range().max,
        full_time_min = viewport.full_time_range().min,
        full_time_max = viewport.full_time_range().max,
        max_bin_count = diagnostics.max_bin_count,
        density_update_cpu_ms = diagnostics.density_update_cpu_duration.as_secs_f64() * 1000.0,
        "RawScope timeline-density viewport updated"
    );
}
