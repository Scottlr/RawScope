//! Timeline-density setup and time-axis interaction.

use std::error::Error;

use rawscope_data::{generate_synthetic_events, load_timeline_dataset, SyntheticEventConfig};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    timeline_marginal_summary, timeline_overview_summary, DensityEncoding,
    ScatterBrushOverlayRenderer, TimelineDensityRenderer, TimelineDensityRendererConfig,
    TimelineViewport,
};
use tracing::info;
use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

use crate::{
    app::{
        WorkbenchApp, DEMO_SEED, TIMELINE_OVERVIEW_BIN_COUNT, WHEEL_ZOOM_IN_SCALE,
        WHEEL_ZOOM_OUT_SCALE,
    },
    app_dataset_profile::resolve_timeline_input_binding,
    cli::WorkbenchInput,
};

const TIMELINE_DEMO_EVENT_COUNT: usize = 20_000;
const TIMELINE_DEMO_GRID_WIDTH: u32 = 256;

impl WorkbenchApp {
    pub(crate) fn prepare_timeline_demo(&mut self, gpu: &GpuContext) -> Result<(), Box<dyn Error>> {
        self.clear_inspection_presentation();
        self.clear_aggregate_overviews();
        self.active_session = None;

        if let Some(WorkbenchInput::Timeline {
            path,
            time_column,
            lane_column,
            limit,
            profile,
        }) = self.input.clone()
        {
            let resolved_binding = resolve_timeline_input_binding(
                &path,
                time_column.as_deref(),
                lane_column.as_deref(),
                limit,
                profile,
            )?;
            let dataset = load_timeline_dataset(
                &path,
                &resolved_binding.time_column,
                &resolved_binding.lane_column,
                limit,
            )?;
            self.activate_session_context(&dataset.source_rows)?;
            let comparison_source_rows = self.load_timeline_comparison_source_rows(
                &resolved_binding.time_column,
                &resolved_binding.lane_column,
                limit,
            )?;
            let viewport = TimelineViewport::new(dataset.time_range, dataset.lane_count);
            let renderer_config =
                timeline_renderer_config(viewport, self.timeline.density_encoding);
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
                time_column = %resolved_binding.time_column,
                lane_column = %resolved_binding.lane_column,
                dataset_profile = ?resolved_binding.active_profile.map(|profile_id| profile_id.as_str()),
                limit = ?limit,
                event_count = render_stats.event_count,
                lane_count = render_stats.lane_count,
                "RawScope local CSV timeline-density dataset prepared"
            );

            self.dataset_identity = Some(dataset.identity);
            self.workbench_state.active_dataset_profile = resolved_binding.active_profile;
            self.workbench_state.dataset_metadata = Some(
                rawscope_data::SyntheticDatasetMetadata::new(0, render_stats.event_count),
            );
            self.clear_active_selection();
            self.timeline.events = dataset.events;
            self.timeline.source_rows = Some(dataset.source_rows);
            self.set_comparison_source_rows(comparison_source_rows);
            self.timeline.viewport = Some(viewport);
            self.timeline.render_stats = Some(render_stats);
            self.timeline.marginal_summary = Some(timeline_marginal_summary(
                &self.timeline.events,
                viewport.time_range(),
                viewport.lane_count(),
                crate::app::MARGINAL_BIN_COUNT,
            ));
            self.timeline.overview_summary = Some(timeline_overview_summary(
                &self.timeline.events,
                viewport.full_time_range(),
                viewport.time_range(),
                TIMELINE_OVERVIEW_BIN_COUNT,
            ));
            self.rebuild_timeline_aggregate_overview();
            self.timeline.density_renderer = Some(timeline_density_renderer);
            self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
            self.scatter_brush_overlay_renderer = Some(ScatterBrushOverlayRenderer::new(
                gpu.device(),
                gpu.surface_format(),
            ));
            self.rebuild_missingness_state();

            return Ok(());
        }

        let dataset = generate_synthetic_events(SyntheticEventConfig::new(
            DEMO_SEED,
            TIMELINE_DEMO_EVENT_COUNT,
        ));
        let viewport = TimelineViewport::new(dataset.time_range, dataset.lane_count);
        let renderer_config = timeline_renderer_config(viewport, self.timeline.density_encoding);
        let timeline_density_renderer = TimelineDensityRenderer::new(
            gpu.device(),
            gpu.queue(),
            gpu.surface_format(),
            &dataset.events,
            renderer_config,
        )?;
        let render_stats = timeline_density_renderer.stats();

        self.dataset_identity = Some(dataset.identity);
        self.workbench_state.active_dataset_profile = None;
        self.workbench_state.dataset_metadata = Some(dataset.metadata);
        self.clear_active_selection();
        self.timeline.events = dataset.events;
        self.timeline.source_rows = None;
        self.clear_dataset_diff_state();
        self.timeline.viewport = Some(viewport);
        self.timeline.render_stats = Some(render_stats);
        self.timeline.marginal_summary = Some(timeline_marginal_summary(
            &self.timeline.events,
            viewport.time_range(),
            viewport.lane_count(),
            crate::app::MARGINAL_BIN_COUNT,
        ));
        self.timeline.overview_summary = Some(timeline_overview_summary(
            &self.timeline.events,
            viewport.full_time_range(),
            viewport.time_range(),
            TIMELINE_OVERVIEW_BIN_COUNT,
        ));
        self.rebuild_timeline_aggregate_overview();
        self.timeline.density_renderer = Some(timeline_density_renderer);
        self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
        self.scatter_brush_overlay_renderer = Some(ScatterBrushOverlayRenderer::new(
            gpu.device(),
            gpu.surface_format(),
        ));
        self.rebuild_missingness_state();

        Ok(())
    }

    pub(crate) fn zoom_timeline_at_cursor(&mut self, scroll_delta: MouseScrollDelta) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let Some((cursor_x_fraction, _y_fraction)) = self.cursor_fraction() else {
            return;
        };
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
        self.begin_timeline_render_refine();
        self.timeline_render_schedule.viewport_changed();
        self.timeline_render_schedule.gesture_released();
        self.request_redraw();
    }

    pub(crate) fn begin_timeline_pan(&mut self) {
        if !self.demo_mode.is_timeline() || self.cursor_fraction().is_none() {
            return;
        }

        self.last_drag_position = self.cursor_position;
        self.begin_timeline_render_refine();
    }

    pub(crate) fn pan_timeline_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let Some(last_drag_position) = self.last_drag_position else {
            return;
        };
        let Some(plot_size) = self.plot_screen_size() else {
            return;
        };
        let Some(viewport) = self.timeline.viewport.as_mut() else {
            return;
        };

        let delta_x_fraction = (position.x - last_drag_position.x) / plot_size.width as f64;

        viewport.pan_by_screen_fraction(delta_x_fraction);
        self.last_drag_position = Some(position);
        self.timeline_render_schedule.viewport_changed();
        self.request_redraw();
    }

    pub(crate) fn end_timeline_pan(&mut self) {
        if self.last_drag_position.is_some() && self.demo_mode.is_timeline() {
            self.timeline_render_schedule.gesture_released();
            self.request_redraw();
        }
        self.last_drag_position = None;
    }

    pub(crate) fn reset_timeline_viewport(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        let Some(viewport) = self.timeline.viewport.as_mut() else {
            return;
        };

        viewport.reset();
        self.timeline_render_schedule.request_exact();
        self.request_redraw();
    }

    fn begin_timeline_render_refine(&mut self) {
        self.cancel_visual_transition();
        self.timeline_render_schedule.gesture_started();
    }

    pub(crate) fn recompute_timeline_density(&mut self) -> Result<(), Box<dyn Error>> {
        self.refresh_timeline_marginal_summary();
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.timeline.viewport else {
            return Ok(());
        };
        let Some(timeline_density_renderer) = self.timeline.density_renderer.as_mut() else {
            return Ok(());
        };

        let renderer_config = timeline_renderer_config(viewport, self.timeline.density_encoding);
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

    fn refresh_timeline_marginal_summary(&mut self) {
        let Some(viewport) = self.timeline.viewport else {
            self.timeline.marginal_summary = None;
            self.timeline.overview_summary = None;
            return;
        };

        self.timeline.marginal_summary = Some(timeline_marginal_summary(
            &self.timeline.events,
            viewport.time_range(),
            viewport.lane_count(),
            crate::app::MARGINAL_BIN_COUNT,
        ));
        self.timeline.overview_summary = Some(timeline_overview_summary(
            &self.timeline.events,
            viewport.full_time_range(),
            viewport.time_range(),
            TIMELINE_OVERVIEW_BIN_COUNT,
        ));
    }
}

fn timeline_renderer_config(
    viewport: TimelineViewport,
    encoding: DensityEncoding,
) -> TimelineDensityRendererConfig {
    TimelineDensityRendererConfig::new(
        viewport.time_range(),
        viewport.lane_count(),
        TIMELINE_DEMO_GRID_WIDTH,
        viewport.lane_count(),
    )
    .with_encoding(encoding)
}
