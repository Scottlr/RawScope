//! Native workbench application handler for RawScope density views.

use std::{error::Error, io, path::PathBuf, sync::Arc};

use egui::Context as EguiContext;
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiWinitState;
use rawscope_data::{
    generate_synthetic_points, load_scatter_dataset, LoadedSourceTable, ScatterPointRecord,
    SyntheticDatasetMetadata, SyntheticPointConfig, TimelineEventRecord,
};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    scatter_marginal_summary, BrushScreenPoint, DensityEncoding, ReliefFieldConfig,
    ScatterAggregateOverview, ScatterBrushDrag, ScatterBrushOverlayRenderer, ScatterBrushSelection,
    ScatterDensityMode, ScatterDensityPresentation, ScatterDensityRenderStats,
    ScatterDensityRenderer, ScatterDensityRendererConfig, ScatterDifferenceRenderStats,
    ScatterDifferenceRenderer, ScatterInspectionOverlayRenderer, ScatterMarginalSummary,
    ScatterSelectionEvidence, ScatterViewport, SelectedRegionSummary, SelectionDrilldown,
    TimelineAggregateOverview, TimelineBrushDrag, TimelineBrushSelection,
    TimelineDensityRenderStats, TimelineDensityRenderer, TimelineMarginalSummary,
    TimelineOverviewSummary, TimelineSelectionEvidence, TimelineSelectionSummary, TimelineViewport,
};
use tracing::info;
use winit::{
    dpi::PhysicalPosition, event_loop::EventLoopProxy, keyboard::ModifiersState, window::Window,
};

use crate::{
    app_dataset_profile::resolve_scatter_input_binding,
    app_inspection_presentation::InspectionPresentationState,
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
    app_session::WorkbenchStartup,
    app_visual_transition::WorkbenchVisualTransition,
    cli::{WorkbenchArgs, WorkbenchInput},
    demo::{DemoMode, PointCountPreset},
    gpu_startup_job::{submit_gpu_initialization, GpuResolution},
    job_coordinator::{JobCoordinator, JobHandle, JobSubmitError, WorkbenchJobId},
    startup_job::{submit_startup_resolution, StartupResolution},
    startup_lifecycle::{StartupLifecycle, StartupRequestId},
    timeline_render_schedule::TimelineRenderSchedule,
    ui_plot_surface::PlotSurfaceLayout,
    ui_shell::WorkbenchShellState,
    workbench_event::WorkbenchUserEvent,
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
    pub(crate) scatter_inspection_overlay_renderer: Option<ScatterInspectionOverlayRenderer>,
    pub(crate) plot_surface: Option<PlotSurfaceLayout>,
    // Selection evidence v1 still serializes synthetic metadata until T005.
    pub(crate) shell: WorkbenchShellState,
    pub(crate) missingness: MissingnessWorkbenchState,
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
    pub(crate) render_schedule: RenderSchedule,
    pub(crate) timeline_render_schedule: TimelineRenderSchedule,
    pub(crate) visual_transition: WorkbenchVisualTransition,
    pub(crate) scatter_filters: ScatterFilterState,
    pub(crate) scatter_inspection: ScatterInspectionState,
    pub(crate) inspection_presentation: InspectionPresentationState,
    pub(crate) point_reveal: ScatterPointRevealState,
    pub(crate) scatter_projection: ScatterProjectionState,
    pub(crate) workbench_state: crate::workbench_state::WorkbenchState,
    pub(crate) startup_coordinator: Option<JobCoordinator>,
    pub(crate) startup_job: Option<(StartupRequestId, JobHandle, StartupResolution)>,
    pub(crate) gpu_job: Option<(JobHandle, GpuResolution)>,
    pub(crate) startup_lifecycle: StartupLifecycle<WorkbenchStartup>,
}

/// Scatter-specific workbench state.
pub(crate) struct ScatterWorkbenchState {
    pub(crate) density_renderer: Option<ScatterDensityRenderer>,
    pub(crate) difference_renderer: Option<ScatterDifferenceRenderer>,
    pub(crate) density_mode: ScatterDensityMode,
    pub(crate) difference_stats: Option<ScatterDifferenceRenderStats>,
    pub(crate) difference_baseline_dirty: bool,
    pub(crate) relief_config: ReliefFieldConfig,
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
    pub(crate) pending_exact_readback: Option<crate::app_render_schedule::PendingExactReadback>,
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
            difference_renderer: None,
            density_mode: ScatterDensityMode::AbsoluteDensity,
            difference_stats: None,
            difference_baseline_dirty: true,
            relief_config: ReliefFieldConfig::default(),
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
            pending_exact_readback: None,
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
    pub(crate) fn new_from_args(
        args: WorkbenchArgs,
        proxy: EventLoopProxy<WorkbenchUserEvent>,
    ) -> Result<Self, JobSubmitError> {
        let startup = crate::app_session::startup_without_session(args.clone());
        let mut app = Self::new(startup.clone());
        let request = StartupRequestId::new(1);
        app.startup_lifecycle.begin(request);
        let coordinator = JobCoordinator::with_proxy(1, proxy);
        if args.session_path.is_some() {
            let (job, resolution) = submit_startup_resolution(&coordinator, args)?;
            app.startup_job = Some((request, job, resolution));
        } else {
            let _ = app.startup_lifecycle.publish_ready(request, startup);
        }
        app.startup_coordinator = Some(coordinator);
        Ok(app)
    }

    pub(crate) fn complete_startup_job(
        &mut self,
        job_id: WorkbenchJobId,
    ) -> Result<bool, io::Error> {
        let Some((request, job, resolution)) = self.startup_job.take() else {
            return Ok(false);
        };
        if job.id() != job_id {
            self.startup_job = Some((request, job, resolution));
            return Ok(false);
        }
        let startup = resolution.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "startup job completed without a durable result",
            )
        })?;
        match startup {
            Ok(startup) => {
                self.demo_mode = startup.demo_mode;
                self.input = startup.input.clone();
                self.compare_input = startup.compare_input.clone();
                self.workbench_state.pending_session = startup.session.clone();
                let _ = self.startup_lifecycle.publish_ready(request, startup);
                Ok(true)
            }
            Err(error) => {
                let _ = self.startup_lifecycle.fail(request, error.to_string());
                Err(error)
            }
        }
    }

    pub(crate) fn start_gpu_initialization(&mut self) -> Result<(), JobSubmitError> {
        let Some(window) = self.window.as_ref().cloned() else {
            return Ok(());
        };
        if self.gpu.is_some() || self.gpu_job.is_some() {
            return Ok(());
        }
        let Some(coordinator) = self.startup_coordinator.as_ref() else {
            return Err(JobSubmitError::ShuttingDown);
        };
        let (job, resolution) = submit_gpu_initialization(coordinator, window)?;
        self.gpu_job = Some((job, resolution));
        Ok(())
    }

    pub(crate) fn complete_gpu_job(&mut self, job_id: WorkbenchJobId) -> Result<bool, io::Error> {
        let Some((job, resolution)) = self.gpu_job.take() else {
            return Ok(false);
        };
        if job.id() != job_id {
            self.gpu_job = Some((job, resolution));
            return Ok(false);
        }
        let gpu = resolution.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "GPU startup job completed without a durable result",
            )
        })??;
        self.gpu = Some(gpu);
        Ok(true)
    }

    pub(crate) fn new(startup: WorkbenchStartup) -> Self {
        Self {
            demo_mode: startup.demo_mode,
            input: startup.input,
            compare_input: startup.compare_input,
            workbench_state: crate::workbench_state::WorkbenchState {
                active_session: None,
                ..crate::workbench_state::WorkbenchState {
                    pending_session: startup.session,
                    ..Default::default()
                }
            },
            scatter: ScatterWorkbenchState {
                point_count_label: PointCountPreset::default().row_count_label().to_string(),
                ..ScatterWorkbenchState::default()
            },
            ..Self::default()
        }
    }

    pub(crate) fn prepare_scatter_demo(&mut self, gpu: &GpuContext) -> Result<(), Box<dyn Error>> {
        self.cancel_visual_transition();
        self.clear_inspection_presentation();
        self.clear_aggregate_overviews();
        self.workbench_state.active_session = None;
        let active_profile = self.input.as_ref().and_then(|input| match input {
            WorkbenchInput::Scatter { profile, .. } => *profile,
            WorkbenchInput::Timeline { .. } => None,
        });
        if let Some(profile_id) = active_profile {
            self.apply_scatter_profile_defaults(profile_id);
        }

        if let Some(WorkbenchInput::Scatter {
            path,
            x_column,
            y_column,
            limit,
            profile,
        }) = self.input.clone()
        {
            let resolved_binding = resolve_scatter_input_binding(
                &path,
                x_column.as_deref(),
                y_column.as_deref(),
                limit,
                profile,
            )?;
            let dataset = load_scatter_dataset(
                &path,
                &resolved_binding.x_column,
                &resolved_binding.y_column,
                limit,
            )?;
            self.activate_session_context(&dataset.source_rows)?;
            let comparison_source_rows = self.load_scatter_comparison_source_rows(
                &resolved_binding.x_column,
                &resolved_binding.y_column,
                limit,
            )?;
            let viewport = ScatterViewport::new(dataset.x_range, dataset.y_range);
            let renderer_config = ScatterDensityRendererConfig::new(
                viewport.x_range(),
                viewport.y_range(),
                DEMO_GRID_WIDTH,
                DEMO_GRID_HEIGHT,
            )
            .with_encoding(self.scatter.density_encoding)
            .with_presentation(self.scatter.density_presentation)
            .with_relief(self.scatter.relief_config);
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
            self.initialize_scatter_difference(gpu, &dataset.points, renderer_config)?;

            self.workbench_state.dataset_identity = Some(dataset.identity);
            self.workbench_state.active_dataset_profile = resolved_binding.active_profile;
            self.workbench_state.dataset_metadata =
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
            self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
            self.scatter_brush_overlay_renderer = Some(scatter_brush_overlay_renderer);
            self.scatter_inspection_overlay_renderer = Some(ScatterInspectionOverlayRenderer::new(
                gpu.device(),
                gpu.surface_format(),
            ));
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
        .with_presentation(self.scatter.density_presentation)
        .with_relief(self.scatter.relief_config);
        let scatter_density_renderer = ScatterDensityRenderer::new(
            gpu.device(),
            gpu.queue(),
            gpu.surface_format(),
            &dataset.points,
            renderer_config,
        )?;
        let scatter_brush_overlay_renderer =
            ScatterBrushOverlayRenderer::new(gpu.device(), gpu.surface_format());
        self.initialize_scatter_difference(gpu, &dataset.points, renderer_config)?;
        let render_stats = scatter_density_renderer.stats();
        info!(
            seed = DEMO_SEED,
            point_count = render_stats.point_count,
            "RawScope synthetic scatter-density demo prepared"
        );

        self.scatter.active_preset = active_preset;
        self.scatter.point_count_label = active_preset.row_count_label().to_string();
        self.workbench_state.dataset_identity = Some(dataset.identity);
        self.workbench_state.active_dataset_profile = None;
        self.workbench_state.dataset_metadata = Some(dataset.metadata);
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
        self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
        self.scatter_brush_overlay_renderer = Some(scatter_brush_overlay_renderer);
        self.scatter_inspection_overlay_renderer = Some(ScatterInspectionOverlayRenderer::new(
            gpu.device(),
            gpu.surface_format(),
        ));
        self.initialize_scatter_point_reveal(gpu);
        self.rebuild_missingness_state();

        Ok(())
    }
}
