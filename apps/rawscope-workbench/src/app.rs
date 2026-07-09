//! Native workbench application handler for RawScope density views.

use std::{error::Error, sync::Arc};

use egui::Context as EguiContext;
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiWinitState;
use rawscope_core::SelectionId;
use rawscope_data::{
    generate_synthetic_points, load_scatter_dataset, DatasetIdentity, LoadedSourceTable,
    ScatterPointRecord, SyntheticDatasetMetadata, SyntheticPointConfig, TimelineEventRecord,
};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    ScatterBrushDrag, ScatterBrushOverlayRenderer, ScatterBrushSelection,
    ScatterDensityRenderStats, ScatterDensityRenderer, ScatterDensityRendererConfig,
    ScatterSelectionEvidence, ScatterViewport, SelectedRegionSummary, SelectionDrilldown,
    TimelineBrushDrag, TimelineBrushSelection, TimelineDensityRenderStats, TimelineDensityRenderer,
    TimelineSelectionEvidence, TimelineSelectionSummary, TimelineViewport,
};
use tracing::{error, info};
use winit::{
    dpi::PhysicalPosition, event::MouseScrollDelta, keyboard::ModifiersState, window::Window,
};

use crate::{
    app_missingness::MissingnessWorkbenchState,
    app_selection::ActiveLinkedSelection,
    cli::{WorkbenchArgs, WorkbenchInput},
    demo::{DemoMode, PointCountPreset},
    ui::{ExportStatus, WorkbenchSurface},
};

pub(crate) const WINDOW_TITLE: &str = "RawScope Workbench";
pub(crate) const INITIAL_WIDTH: f64 = 1280.0;
pub(crate) const INITIAL_HEIGHT: f64 = 720.0;
pub(crate) const DEMO_SEED: u64 = 42;
pub(crate) const DEMO_GRID_WIDTH: u32 = 256;
pub(crate) const DEMO_GRID_HEIGHT: u32 = 256;
pub(crate) const WHEEL_ZOOM_IN_SCALE: f32 = 0.82;
pub(crate) const WHEEL_ZOOM_OUT_SCALE: f32 = 1.22;

/// Winit application state for RawScope density views.
#[derive(Default)]
pub struct WorkbenchApp {
    pub(crate) demo_mode: DemoMode,
    pub(crate) input: Option<WorkbenchInput>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) gpu: Option<GpuContext>,
    pub(crate) egui_context: EguiContext,
    pub(crate) egui_state: Option<EguiWinitState>,
    pub(crate) egui_renderer: Option<EguiRenderer>,
    pub(crate) scatter_brush_overlay_renderer: Option<ScatterBrushOverlayRenderer>,
    pub(crate) dataset_identity: Option<DatasetIdentity>,
    // Selection evidence v1 still serializes synthetic metadata until T005.
    pub(crate) dataset_metadata: Option<SyntheticDatasetMetadata>,
    pub(crate) active_selection: Option<ActiveLinkedSelection>,
    pub(crate) next_selection_id: SelectionId,
    pub(crate) visible_surface: WorkbenchSurface,
    pub(crate) export_status: ExportStatus,
    pub(crate) missingness: MissingnessWorkbenchState,
    pub(crate) scatter: ScatterWorkbenchState,
    pub(crate) timeline: TimelineWorkbenchState,
    pub(crate) cursor_position: Option<PhysicalPosition<f64>>,
    pub(crate) last_drag_position: Option<PhysicalPosition<f64>>,
    pub(crate) modifiers: ModifiersState,
    pub(crate) evidence_export_counter: u64,
}

/// Scatter-specific workbench state.
#[derive(Default)]
pub(crate) struct ScatterWorkbenchState {
    pub(crate) density_renderer: Option<ScatterDensityRenderer>,
    pub(crate) points: Vec<ScatterPointRecord>,
    pub(crate) source_rows: Option<LoadedSourceTable>,
    pub(crate) active_preset: PointCountPreset,
    pub(crate) point_count_label: String,
    pub(crate) viewport: Option<ScatterViewport>,
    pub(crate) render_stats: Option<ScatterDensityRenderStats>,
    pub(crate) brush_drag_start: Option<PhysicalPosition<f64>>,
    pub(crate) active_brush_drag: Option<ScatterBrushDrag>,
    pub(crate) active_brush_selection: Option<ScatterBrushSelection>,
    pub(crate) selection_summary: Option<SelectedRegionSummary>,
    pub(crate) selection_evidence: Option<ScatterSelectionEvidence>,
    pub(crate) selection_drilldown: Option<SelectionDrilldown>,
}

/// Timeline-specific workbench state.
#[derive(Default)]
pub(crate) struct TimelineWorkbenchState {
    pub(crate) density_renderer: Option<TimelineDensityRenderer>,
    pub(crate) events: Vec<TimelineEventRecord>,
    pub(crate) source_rows: Option<LoadedSourceTable>,
    pub(crate) viewport: Option<TimelineViewport>,
    pub(crate) render_stats: Option<TimelineDensityRenderStats>,
    pub(crate) brush_drag_start: Option<PhysicalPosition<f64>>,
    pub(crate) active_brush_drag: Option<TimelineBrushDrag>,
    pub(crate) active_brush_selection: Option<TimelineBrushSelection>,
    pub(crate) selection_summary: Option<TimelineSelectionSummary>,
    pub(crate) selection_evidence: Option<TimelineSelectionEvidence>,
    pub(crate) selection_drilldown: Option<SelectionDrilldown>,
}

impl WorkbenchApp {
    pub(crate) fn new(args: WorkbenchArgs) -> Self {
        Self {
            demo_mode: args.demo_mode,
            input: args.input,
            scatter: ScatterWorkbenchState {
                point_count_label: PointCountPreset::default().row_count_label().to_string(),
                ..ScatterWorkbenchState::default()
            },
            ..Self::default()
        }
    }

    pub(crate) fn prepare_scatter_demo(&mut self, gpu: &GpuContext) -> Result<(), Box<dyn Error>> {
        if let Some(WorkbenchInput::Scatter {
            path,
            x_column,
            y_column,
            limit,
        }) = self.input.as_ref()
        {
            let dataset = load_scatter_dataset(path, x_column, y_column, *limit)?;
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
            let scatter_brush_overlay_renderer =
                ScatterBrushOverlayRenderer::new(gpu.device(), gpu.surface_format());
            let render_stats = scatter_density_renderer.stats();
            info!(
                path = %path.display(),
                x_column,
                y_column,
                limit = ?limit,
                row_count = render_stats.point_count,
                "RawScope local CSV scatter-density dataset prepared"
            );

            self.dataset_identity = Some(dataset.identity);
            self.dataset_metadata =
                Some(SyntheticDatasetMetadata::new(0, render_stats.point_count));
            self.clear_active_selection();
            self.scatter.points = dataset.points;
            self.scatter.source_rows = Some(dataset.source_rows);
            self.scatter.point_count_label = "local".to_string();
            self.scatter.viewport = Some(viewport);
            self.scatter.render_stats = Some(render_stats);
            self.scatter.density_renderer = Some(scatter_density_renderer);
            self.export_status = crate::ui::ExportStatus::Idle;
            self.scatter_brush_overlay_renderer = Some(scatter_brush_overlay_renderer);
            self.rebuild_missingness_state();

            return Ok(());
        }

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
        let scatter_brush_overlay_renderer =
            ScatterBrushOverlayRenderer::new(gpu.device(), gpu.surface_format());
        let render_stats = scatter_density_renderer.stats();
        info!(
            seed = DEMO_SEED,
            point_count = render_stats.point_count,
            "RawScope synthetic scatter-density demo prepared"
        );

        self.scatter.active_preset = active_preset;
        self.scatter.point_count_label = active_preset.row_count_label().to_string();
        self.dataset_identity = Some(dataset.identity);
        self.dataset_metadata = Some(dataset.metadata);
        self.clear_active_selection();
        self.scatter.points = dataset.points;
        self.scatter.source_rows = None;
        self.scatter.viewport = Some(viewport);
        self.scatter.render_stats = Some(render_stats);
        self.scatter.density_renderer = Some(scatter_density_renderer);
        self.export_status = crate::ui::ExportStatus::Idle;
        self.scatter_brush_overlay_renderer = Some(scatter_brush_overlay_renderer);
        self.rebuild_missingness_state();

        Ok(())
    }

    pub(crate) fn switch_point_preset(&mut self, preset: PointCountPreset) {
        if !self.demo_mode.is_scatter() {
            return;
        }
        if self.input.is_some() {
            return;
        }

        let preset_is_already_active = self.scatter.active_preset == preset;
        if preset_is_already_active {
            return;
        }

        let dataset =
            generate_synthetic_points(SyntheticPointConfig::new(DEMO_SEED, preset.row_count));
        let viewport = ScatterViewport::new(dataset.x_range, dataset.y_range);
        self.scatter.points = dataset.points;
        self.scatter.source_rows = None;
        self.scatter.active_preset = preset;
        self.scatter.point_count_label = preset.row_count_label().to_string();
        self.dataset_identity = Some(dataset.identity);
        self.dataset_metadata = Some(dataset.metadata);
        self.clear_active_selection();
        self.scatter.viewport = Some(viewport);
        self.export_status = crate::ui::ExportStatus::Idle;
        self.clear_brush();
        self.rebuild_missingness_state();

        if let Err(err) = self.recompute_density() {
            error!(
                error = %err,
                point_count = preset.row_count,
                "failed to recompute scatter density after preset change"
            );
        }
    }

    pub(crate) fn zoom_at_cursor(&mut self, scroll_delta: MouseScrollDelta) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        let cursor_fraction = self.cursor_fraction();
        let Some(viewport) = self.scatter.viewport.as_mut() else {
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
        if let Err(err) = self.recompute_density() {
            error!(error = %err, "failed to recompute scatter density after zoom");
        }
    }

    pub(crate) fn begin_pan(&mut self) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        self.last_drag_position = self.cursor_position;
    }

    pub(crate) fn end_pan(&mut self) {
        self.last_drag_position = None;
    }

    pub(crate) fn pan_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        let Some(last_drag_position) = self.last_drag_position else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.scatter.viewport.as_mut() else {
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
        if let Err(err) = self.recompute_density() {
            error!(error = %err, "failed to recompute scatter density after pan");
        }
    }

    pub(crate) fn reset_viewport(&mut self) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        let Some(viewport) = self.scatter.viewport.as_mut() else {
            return;
        };

        viewport.reset();
        if let Err(err) = self.recompute_density() {
            error!(error = %err, "failed to recompute scatter density after reset");
        }
    }

    fn recompute_density(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.scatter.viewport else {
            return Ok(());
        };
        let Some(scatter_density_renderer) = self.scatter.density_renderer.as_mut() else {
            return Ok(());
        };

        let renderer_config = ScatterDensityRendererConfig::new(
            viewport.x_range(),
            viewport.y_range(),
            DEMO_GRID_WIDTH,
            DEMO_GRID_HEIGHT,
        );
        let stats = scatter_density_renderer.update_density(
            gpu.device(),
            gpu.queue(),
            &self.scatter.points,
            renderer_config,
        )?;
        self.scatter.render_stats = Some(stats);
        self.update_window_title();
        self.request_redraw();

        Ok(())
    }
}
