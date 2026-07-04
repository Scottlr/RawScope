//! Timeline-density setup and time-axis interaction.

use std::error::Error;

use rawscope_data::{generate_synthetic_events, load_timeline_dataset, SyntheticEventConfig};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    ScatterBrushOverlayRenderer, TimelineDensityRenderer, TimelineDensityRendererConfig,
    TimelineViewport,
};
use tracing::{error, info};
use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

use crate::{
    app::{WorkbenchApp, DEMO_SEED, WHEEL_ZOOM_IN_SCALE, WHEEL_ZOOM_OUT_SCALE},
    cli::WorkbenchInput,
};

const TIMELINE_DEMO_EVENT_COUNT: usize = 20_000;
const TIMELINE_DEMO_GRID_WIDTH: u32 = 256;

impl WorkbenchApp {
    pub(crate) fn prepare_timeline_demo(&mut self, gpu: &GpuContext) -> Result<(), Box<dyn Error>> {
        if let Some(WorkbenchInput::Timeline {
            path,
            time_column,
            lane_column,
            limit,
        }) = self.input.as_ref()
        {
            let dataset = load_timeline_dataset(path, time_column, lane_column, *limit)?;
            let viewport = TimelineViewport::new(dataset.time_range, dataset.lane_count);
            let renderer_config = timeline_renderer_config(viewport);
            let timeline_density_renderer = TimelineDensityRenderer::new(
                gpu.device(),
                gpu.queue(),
                gpu.surface_format(),
                &dataset.events,
                renderer_config,
            )?;
            let render_stats = timeline_density_renderer.stats();
            info!(
                path = %path.display(),
                time_column,
                lane_column,
                limit = ?limit,
                event_count = render_stats.event_count,
                lane_count = render_stats.lane_count,
                "RawScope local CSV timeline-density dataset prepared"
            );

            self.dataset_metadata = Some(rawscope_data::SyntheticDatasetMetadata::new(
                0,
                render_stats.event_count,
            ));
            self.timeline.events = dataset.events;
            self.timeline.viewport = Some(viewport);
            self.timeline.render_stats = Some(render_stats);
            self.timeline.density_renderer = Some(timeline_density_renderer);
            self.scatter_brush_overlay_renderer = Some(ScatterBrushOverlayRenderer::new(
                gpu.device(),
                gpu.surface_format(),
            ));

            return Ok(());
        }

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
        let render_stats = timeline_density_renderer.stats();

        self.dataset_metadata = Some(dataset.metadata);
        self.timeline.events = dataset.events;
        self.timeline.viewport = Some(viewport);
        self.timeline.render_stats = Some(render_stats);
        self.timeline.density_renderer = Some(timeline_density_renderer);
        self.scatter_brush_overlay_renderer = Some(ScatterBrushOverlayRenderer::new(
            gpu.device(),
            gpu.surface_format(),
        ));

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
        let Some(viewport) = self.timeline.viewport.as_mut() else {
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
        if let Err(err) = self.recompute_timeline_density() {
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
        let Some(viewport) = self.timeline.viewport.as_mut() else {
            return;
        };

        let window_size = window.inner_size();
        let window_has_width = window_size.width > 0;
        if !window_has_width {
            return;
        }

        let delta_x_fraction = (position.x - last_drag_position.x) / window_size.width as f64;

        viewport.pan_by_screen_fraction(delta_x_fraction);
        self.last_drag_position = Some(position);
        if let Err(err) = self.recompute_timeline_density() {
            error!(error = %err, "failed to recompute timeline density after pan");
        }
    }

    pub(crate) fn reset_timeline_viewport(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let Some(viewport) = self.timeline.viewport.as_mut() else {
            return;
        };

        viewport.reset();
        if let Err(err) = self.recompute_timeline_density() {
            error!(error = %err, "failed to recompute timeline density after reset");
        }
    }

    fn recompute_timeline_density(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.timeline.viewport else {
            return Ok(());
        };
        let Some(timeline_density_renderer) = self.timeline.density_renderer.as_mut() else {
            return Ok(());
        };

        let renderer_config = timeline_renderer_config(viewport);
        let stats = timeline_density_renderer.update_density(
            gpu.device(),
            gpu.queue(),
            &self.timeline.events,
            renderer_config,
        )?;
        self.timeline.render_stats = Some(stats);
        self.update_window_title();
        self.request_redraw();

        Ok(())
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
