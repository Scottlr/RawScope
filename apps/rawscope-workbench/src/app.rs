//! Native workbench application handler for RawScope density views.

use std::{error::Error, path::PathBuf, sync::Arc};

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
    scatter_marginal_summary, scatter_marginal_summary_masked, BrushScreenPoint,
    DatasetDiffSummary, DensityEncoding, DensityReadbackPolicy, ScatterAggregateOverview,
    ScatterBrushDrag, ScatterBrushOverlayRenderer, ScatterBrushSelection,
    ScatterDensityPresentation, ScatterDensityRenderStats, ScatterDensityRenderer,
    ScatterDensityRendererConfig, ScatterDensityUpdate, ScatterMarginalSummary,
    ScatterSelectionEvidence, ScatterViewport, SelectedRegionSummary, SelectionDrilldown,
    TimelineAggregateOverview, TimelineBrushDrag, TimelineBrushSelection,
    TimelineDensityRenderStats, TimelineDensityRenderer, TimelineMarginalSummary,
    TimelineOverviewSummary, TimelineSelectionEvidence, TimelineSelectionSummary, TimelineViewport,
};
use tracing::{error, info};
use winit::{dpi::PhysicalPosition, keyboard::ModifiersState, window::Window};

use crate::{
    app_dataset_profile::resolve_scatter_input_binding,
    app_interaction_mode::{
        ActivePointerGesture, InspectCursorPosition, InteractionOverride, SelectionGestureBackup,
        WorkbenchInteractionMode,
    },
    app_missingness::MissingnessWorkbenchState,
    app_render_schedule::RenderSchedule,
    app_scatter_filter::ScatterFilterState,
    app_scatter_inspection::ScatterInspectionState,
    app_scatter_point_reveal::ScatterPointRevealState,
    app_scatter_projection::ScatterProjectionState,
    app_selection::ActiveLinkedSelection,
    cli::{WorkbenchArgs, WorkbenchInput},
    demo::{DemoMode, PointCountPreset},
    ui::{ExportStatus, WorkbenchSurface},
    ui_plot_surface::PlotSurfaceLayout,
    ui_shell::WorkbenchShellState,
};

pub(crate) const WINDOW_TITLE: &str = "RawScope Workbench";
pub(crate) const INITIAL_WIDTH: f64 = 1280.0;
pub(crate) const INITIAL_HEIGHT: f64 = 720.0;
pub(crate) const DEMO_SEED: u64 = 42;
pub(crate) const DEMO_GRID_WIDTH: u32 = 256;
pub(crate) const DEMO_GRID_HEIGHT: u32 = 256;
pub(crate) const MARGINAL_BIN_COUNT: u32 = 64;
pub(crate) const TIMELINE_OVERVIEW_BIN_COUNT: u32 = 128;
pub(crate) const WHEEL_ZOOM_IN_SCALE: f32 = 0.82;
pub(crate) const WHEEL_ZOOM_OUT_SCALE: f32 = 1.22;

/// Winit application state for RawScope density views.
#[derive(Default)]
pub struct WorkbenchApp {
    pub(crate) demo_mode: DemoMode,
    pub(crate) input: Option<WorkbenchInput>,
    pub(crate) compare_input: Option<PathBuf>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) gpu: Option<GpuContext>,
    pub(crate) egui_context: EguiContext,
    pub(crate) egui_state: Option<EguiWinitState>,
    pub(crate) egui_renderer: Option<EguiRenderer>,
    pub(crate) scatter_brush_overlay_renderer: Option<ScatterBrushOverlayRenderer>,
    pub(crate) plot_surface: Option<PlotSurfaceLayout>,
    pub(crate) dataset_identity: Option<DatasetIdentity>,
    pub(crate) active_dataset_profile: Option<rawscope_data::DatasetProfileId>,
    // Selection evidence v1 still serializes synthetic metadata until T005.
    pub(crate) dataset_metadata: Option<SyntheticDatasetMetadata>,
    pub(crate) active_selection: Option<ActiveLinkedSelection>,
    pub(crate) active_comparison: Option<crate::app_comparison::WorkbenchComparison>,
    pub(crate) next_selection_id: SelectionId,
    pub(crate) visible_surface: WorkbenchSurface,
    pub(crate) export_status: ExportStatus,
    pub(crate) shell: WorkbenchShellState,
    pub(crate) missingness: MissingnessWorkbenchState,
    pub(crate) comparison_source_rows: Option<LoadedSourceTable>,
    pub(crate) dataset_diff_summary: Option<DatasetDiffSummary>,
    pub(crate) scatter: ScatterWorkbenchState,
    pub(crate) timeline: TimelineWorkbenchState,
    pub(crate) cursor_position: Option<PhysicalPosition<f64>>,
    pub(crate) last_drag_position: Option<PhysicalPosition<f64>>,
    pub(crate) interaction_mode: WorkbenchInteractionMode,
    pub(crate) active_pointer_gesture: Option<ActivePointerGesture>,
    pub(crate) interaction_override: Option<InteractionOverride>,
    pub(crate) active_pointer_button: Option<winit::event::MouseButton>,
    pub(crate) inspect_cursor_position: Option<InspectCursorPosition>,
    pub(crate) selection_gesture_backup: Option<SelectionGestureBackup>,
    pub(crate) modifiers: ModifiersState,
    pub(crate) evidence_export_counter: u64,
    pub(crate) render_schedule: RenderSchedule,
    pub(crate) scatter_filters: ScatterFilterState,
    pub(crate) scatter_inspection: ScatterInspectionState,
    pub(crate) point_reveal: ScatterPointRevealState,
    pub(crate) scatter_projection: ScatterProjectionState,
}

/// Scatter-specific workbench state.
pub(crate) struct ScatterWorkbenchState {
    pub(crate) density_renderer: Option<ScatterDensityRenderer>,
    pub(crate) density_dataset_revision: u64,
    pub(crate) density_encoding: DensityEncoding,
    pub(crate) density_presentation: ScatterDensityPresentation,
    pub(crate) points: Vec<ScatterPointRecord>,
    pub(crate) source_rows: Option<LoadedSourceTable>,
    pub(crate) active_preset: PointCountPreset,
    pub(crate) point_count_label: String,
    pub(crate) viewport: Option<ScatterViewport>,
    pub(crate) render_stats: Option<ScatterDensityRenderStats>,
    pub(crate) marginal_summary: Option<ScatterMarginalSummary>,
    pub(crate) scatter_aggregate_overview: Option<ScatterAggregateOverview>,
    pub(crate) brush_drag_start: Option<BrushScreenPoint>,
    pub(crate) active_brush_drag: Option<ScatterBrushDrag>,
    pub(crate) active_brush_selection: Option<ScatterBrushSelection>,
    pub(crate) selection_summary: Option<SelectedRegionSummary>,
    pub(crate) selection_evidence: Option<ScatterSelectionEvidence>,
    pub(crate) selection_drilldown: Option<SelectionDrilldown>,
}

/// Timeline-specific workbench state.
pub(crate) struct TimelineWorkbenchState {
    pub(crate) density_renderer: Option<TimelineDensityRenderer>,
    pub(crate) density_encoding: DensityEncoding,
    pub(crate) events: Vec<TimelineEventRecord>,
    pub(crate) source_rows: Option<LoadedSourceTable>,
    pub(crate) viewport: Option<TimelineViewport>,
    pub(crate) render_stats: Option<TimelineDensityRenderStats>,
    pub(crate) marginal_summary: Option<TimelineMarginalSummary>,
    pub(crate) overview_summary: Option<TimelineOverviewSummary>,
    pub(crate) timeline_aggregate_overview: Option<TimelineAggregateOverview>,
    pub(crate) brush_drag_start: Option<BrushScreenPoint>,
    pub(crate) active_brush_drag: Option<TimelineBrushDrag>,
    pub(crate) active_brush_selection: Option<TimelineBrushSelection>,
    pub(crate) selection_summary: Option<TimelineSelectionSummary>,
    pub(crate) selection_evidence: Option<TimelineSelectionEvidence>,
    pub(crate) selection_drilldown: Option<SelectionDrilldown>,
}

impl Default for ScatterWorkbenchState {
    fn default() -> Self {
        Self {
            density_renderer: None,
            density_dataset_revision: 0,
            density_encoding: DensityEncoding::scatter_default(),
            density_presentation: ScatterDensityPresentation::TopographicField,
            points: Vec::new(),
            source_rows: None,
            active_preset: PointCountPreset::default(),
            point_count_label: String::new(),
            viewport: None,
            render_stats: None,
            marginal_summary: None,
            scatter_aggregate_overview: None,
            brush_drag_start: None,
            active_brush_drag: None,
            active_brush_selection: None,
            selection_summary: None,
            selection_evidence: None,
            selection_drilldown: None,
        }
    }
}

impl Default for TimelineWorkbenchState {
    fn default() -> Self {
        Self {
            density_renderer: None,
            density_encoding: DensityEncoding::timeline_default(),
            events: Vec::new(),
            source_rows: None,
            viewport: None,
            render_stats: None,
            marginal_summary: None,
            overview_summary: None,
            timeline_aggregate_overview: None,
            brush_drag_start: None,
            active_brush_drag: None,
            active_brush_selection: None,
            selection_summary: None,
            selection_evidence: None,
            selection_drilldown: None,
        }
    }
}

impl WorkbenchApp {
    pub(crate) fn new(args: WorkbenchArgs) -> Self {
        Self {
            demo_mode: args.demo_mode,
            input: args.input,
            compare_input: args.compare_input,
            scatter: ScatterWorkbenchState {
                point_count_label: PointCountPreset::default().row_count_label().to_string(),
                ..ScatterWorkbenchState::default()
            },
            ..Self::default()
        }
    }

    pub(crate) fn prepare_scatter_demo(&mut self, gpu: &GpuContext) -> Result<(), Box<dyn Error>> {
        self.clear_aggregate_overviews();

        if let Some(WorkbenchInput::Scatter {
            path,
            x_column,
            y_column,
            limit,
            profile,
        }) = self.input.as_ref()
        {
            let resolved_binding = resolve_scatter_input_binding(
                path,
                x_column.as_deref(),
                y_column.as_deref(),
                *limit,
                *profile,
            )?;
            let dataset = load_scatter_dataset(
                path,
                &resolved_binding.x_column,
                &resolved_binding.y_column,
                *limit,
            )?;
            let comparison_source_rows = self.load_scatter_comparison_source_rows(
                &resolved_binding.x_column,
                &resolved_binding.y_column,
                *limit,
            )?;
            let viewport = ScatterViewport::new(dataset.x_range, dataset.y_range);
            let renderer_config = ScatterDensityRendererConfig::new(
                viewport.x_range(),
                viewport.y_range(),
                DEMO_GRID_WIDTH,
                DEMO_GRID_HEIGHT,
            )
            .with_encoding(self.scatter.density_encoding)
            .with_presentation(self.scatter.density_presentation);
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
                x_column = %resolved_binding.x_column,
                y_column = %resolved_binding.y_column,
                dataset_profile = ?resolved_binding.active_profile.map(|profile_id| profile_id.as_str()),
                limit = ?limit,
                row_count = render_stats.point_count,
                "RawScope local CSV scatter-density dataset prepared"
            );

            self.dataset_identity = Some(dataset.identity);
            self.active_dataset_profile = resolved_binding.active_profile;
            self.dataset_metadata =
                Some(SyntheticDatasetMetadata::new(0, render_stats.point_count));
            self.clear_active_selection();
            self.scatter.points = dataset.points;
            self.scatter.source_rows = Some(dataset.source_rows);
            self.initialize_scatter_filters();
            self.initialize_scatter_projection(
                &resolved_binding.x_column,
                &resolved_binding.y_column,
            );
            self.set_comparison_source_rows(comparison_source_rows);
            self.scatter.point_count_label = "local".to_string();
            self.scatter.viewport = Some(viewport);
            self.scatter.render_stats = Some(render_stats);
            self.scatter.marginal_summary = Some(scatter_marginal_summary(
                &self.scatter.points,
                viewport.x_range(),
                viewport.y_range(),
                MARGINAL_BIN_COUNT,
                MARGINAL_BIN_COUNT,
            ));
            self.rebuild_scatter_aggregate_overview();
            self.scatter.density_renderer = Some(scatter_density_renderer);
            self.rebuild_scatter_inspection_cache();
            self.export_status = crate::ui::ExportStatus::Idle;
            self.scatter_brush_overlay_renderer = Some(scatter_brush_overlay_renderer);
            self.initialize_scatter_point_reveal(gpu);
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
        )
        .with_encoding(self.scatter.density_encoding)
        .with_presentation(self.scatter.density_presentation);
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
        self.active_dataset_profile = None;
        self.dataset_metadata = Some(dataset.metadata);
        self.clear_active_selection();
        self.scatter.points = dataset.points;
        self.scatter.source_rows = None;
        self.scatter_filters = ScatterFilterState::default();
        self.scatter_inspection = ScatterInspectionState::default();
        self.scatter_projection = ScatterProjectionState::default();
        self.clear_dataset_diff_state();
        self.scatter.viewport = Some(viewport);
        self.scatter.render_stats = Some(render_stats);
        self.scatter.marginal_summary = Some(scatter_marginal_summary(
            &self.scatter.points,
            viewport.x_range(),
            viewport.y_range(),
            MARGINAL_BIN_COUNT,
            MARGINAL_BIN_COUNT,
        ));
        self.rebuild_scatter_aggregate_overview();
        self.scatter.density_renderer = Some(scatter_density_renderer);
        self.export_status = crate::ui::ExportStatus::Idle;
        self.scatter_brush_overlay_renderer = Some(scatter_brush_overlay_renderer);
        self.initialize_scatter_point_reveal(gpu);
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
        self.scatter_filters = ScatterFilterState::default();
        self.scatter_inspection = ScatterInspectionState::default();
        self.scatter_projection = ScatterProjectionState::default();
        self.reset_scatter_point_reveal_mask();
        self.clear_dataset_diff_state();
        self.scatter.active_preset = preset;
        self.scatter.point_count_label = preset.row_count_label().to_string();
        self.dataset_identity = Some(dataset.identity);
        self.active_dataset_profile = None;
        self.dataset_metadata = Some(dataset.metadata);
        self.clear_active_selection();
        self.scatter.viewport = Some(viewport);
        self.export_status = crate::ui::ExportStatus::Idle;
        self.clear_brush();
        self.rebuild_missingness_state();
        self.rebuild_scatter_aggregate_overview();

        self.scatter.density_dataset_revision += 1;
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.density_renderer.as_mut())
        {
            if let Err(err) = renderer.replace_dataset(
                gpu.device(),
                gpu.queue(),
                &self.scatter.points,
                self.scatter.density_dataset_revision,
            ) {
                error!(error = %err, "failed to replace resident scatter dataset");
                return;
            }
        }

        if let Err(err) = self.recompute_density() {
            error!(
                error = %err,
                point_count = preset.row_count,
                "failed to recompute scatter density after preset change"
            );
        }
    }

    pub(crate) fn recompute_density(&mut self) -> Result<(), Box<dyn Error>> {
        self.refresh_scatter_marginal_summary();
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
        )
        .with_encoding(self.scatter.density_encoding)
        .with_presentation(self.scatter.density_presentation);
        let stats = scatter_density_renderer.update_density(
            gpu.device(),
            gpu.queue(),
            ScatterDensityUpdate {
                config: renderer_config,
                readback: DensityReadbackPolicy::None,
            },
        )?;
        self.scatter.render_stats = Some(stats);
        self.render_schedule.exact_field_settled();
        self.rebuild_scatter_inspection_cache();
        self.update_window_title();
        self.request_redraw();

        Ok(())
    }

    pub(crate) fn refresh_scatter_marginal_summary(&mut self) {
        let Some(viewport) = self.scatter.viewport else {
            self.scatter.marginal_summary = None;
            return;
        };

        self.scatter.marginal_summary = self
            .scatter_filters
            .evaluation
            .as_ref()
            .map(|evaluation| {
                scatter_marginal_summary_masked(
                    &self.scatter.points,
                    &evaluation.mask,
                    viewport.x_range(),
                    viewport.y_range(),
                    MARGINAL_BIN_COUNT,
                    MARGINAL_BIN_COUNT,
                )
                .expect("filter evaluation remains aligned with scatter points")
            })
            .or_else(|| {
                Some(scatter_marginal_summary(
                    &self.scatter.points,
                    viewport.x_range(),
                    viewport.y_range(),
                    MARGINAL_BIN_COUNT,
                    MARGINAL_BIN_COUNT,
                ))
            });
    }
}
