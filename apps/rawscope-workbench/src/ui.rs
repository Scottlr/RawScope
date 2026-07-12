//! Visible workbench UI state and formatting helpers.

use egui::ViewportId;
use egui_wgpu::RendererOptions;
use rawscope_core::VisualSelectionKind;
use rawscope_render::{
    DatasetDiffSummary, MissingnessGrid, MissingnessSelection, MissingnessSelectionSummary,
    SelectionDrilldown,
};

use crate::{
    app::WorkbenchApp,
    app_comparison::WorkbenchComparison,
    app_interaction_mode::{InspectCursorPosition, WorkbenchInteractionMode},
    demo::DemoMode,
    ui_controls::UiActions,
    ui_dataset_identity::DatasetDisplayIdentity,
    ui_filters::{scatter_filters_ui_state, ScatterFiltersUiState},
    ui_inspection_tooltip::InspectionPresentationFrame,
    ui_scatter_inspection::{scatter_inspection_ui_state, ScatterInspectionUiState},
    ui_shell::{show_workbench_ui, WorkbenchShellState, WorkbenchUiOutput},
    ui_view_context::{view_axes_ui_state, view_context_ui_state, WorkbenchViewContextUiState},
    ui_visual_encoding::{density_encoding_ui_state, DensityEncodingUiState},
};

/// Active visual workbench view shown in the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveView {
    Scatter,
    Timeline,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WorkbenchViewAxes {
    Scatter(rawscope_render::ScatterAxesContext),
    Timeline(rawscope_render::TimelineAxesContext),
}

/// Visible analyst surface shown above the current dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkbenchSurface {
    #[default]
    Primary,
    Missingness,
    DatasetDiff,
}

/// UI-ready projection of the current missingness slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingnessUiState {
    pub(crate) grid: MissingnessGrid,
    pub(crate) selection: Option<MissingnessSelection>,
    pub(crate) selection_summary: Option<MissingnessSelectionSummary>,
    pub(crate) column_names: Vec<String>,
    pub(crate) row_bucket_labels: Vec<String>,
}

/// UI-ready projection of the current bounded dataset diff.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DatasetDiffUiState {
    pub(crate) summary: DatasetDiffSummary,
    pub(crate) before_label: String,
    pub(crate) after_label: String,
}

/// User-visible export status for the current workbench session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ExportStatus {
    #[default]
    Idle,
    NoSelection,
    Exported {
        bundle_dir: String,
    },
    Failed {
        message: String,
    },
}

impl ExportStatus {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Idle => "Export idle".to_string(),
            Self::NoSelection => "Export skipped: no finalized selection".to_string(),
            Self::Exported { bundle_dir } => format!("Exported bundle {bundle_dir}"),
            Self::Failed { message } => format!("Export failed: {message}"),
        }
    }
}

/// UI-ready projection of the current workbench session.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkbenchUiState {
    pub(crate) active_view: ActiveView,
    pub(crate) visible_surface: WorkbenchSurface,
    pub(crate) dataset_identity: DatasetDisplayIdentity,
    pub(crate) shell: WorkbenchShellState,
    pub(crate) interaction_mode: WorkbenchInteractionMode,
    pub(crate) inspect_cursor_position: Option<InspectCursorPosition>,
    pub(crate) view_label: String,
    pub(crate) selection_label: String,
    pub(crate) linked_selection_label: String,
    pub(crate) comparison: Option<WorkbenchComparison>,
    pub(crate) axis_primary_label: String,
    pub(crate) axis_secondary_label: String,
    pub(crate) view_axes: Option<WorkbenchViewAxes>,
    pub(crate) density_encoding: Option<DensityEncodingUiState>,
    pub(crate) view_context: Option<WorkbenchViewContextUiState>,
    pub(crate) export_status: ExportStatus,
    pub(crate) drilldown: Option<SelectionDrilldown>,
    pub(crate) missingness: Option<MissingnessUiState>,
    pub(crate) dataset_diff: Option<DatasetDiffUiState>,
    pub(crate) can_switch_to_scatter: bool,
    pub(crate) can_switch_to_timeline: bool,
    pub(crate) can_show_missingness: bool,
    pub(crate) can_show_dataset_diff: bool,
    pub(crate) can_reset: bool,
    pub(crate) can_export: bool,
    pub(crate) can_clear_selection: bool,
    pub(crate) density_is_refining: bool,
    pub(crate) reduced_motion: bool,
    pub(crate) scatter_filters: Option<ScatterFiltersUiState>,
    pub(crate) active_cohort_row_count: usize,
    pub(crate) scatter_inspection: Option<ScatterInspectionUiState>,
    pub(crate) inspection_presentation: Option<ScatterInspectionUiState>,
    pub(crate) inspection_presentation_frame: InspectionPresentationFrame,
}

impl WorkbenchUiState {
    pub(crate) fn window_title(&self) -> String {
        self.dataset_identity.window_title()
    }
}

impl ActiveView {
    pub(crate) fn from_demo_mode(mode: DemoMode) -> Self {
        match mode {
            DemoMode::Scatter => Self::Scatter,
            DemoMode::Timeline => Self::Timeline,
        }
    }
}

impl WorkbenchApp {
    pub(crate) fn initialize_ui_integration(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };

        crate::ui_theme::apply_theme(&self.egui_context);
        let max_texture_side = Some(gpu.device().limits().max_texture_dimension_2d as usize);
        self.egui_state = Some(egui_winit::State::new(
            self.egui_context.clone(),
            ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            max_texture_side,
        ));
        self.egui_renderer = Some(egui_wgpu::Renderer::new(
            gpu.device(),
            gpu.surface_format(),
            RendererOptions::default(),
        ));
    }

    pub(crate) fn ui_state(&self) -> WorkbenchUiState {
        let active_view = ActiveView::from_demo_mode(self.demo_mode);
        let visible_surface = self.workbench_state.visible_surface;
        let dataset_identity = self
            .workbench_state
            .dataset_identity
            .as_ref()
            .map(|identity| {
                DatasetDisplayIdentity::from_dataset_with_display_name(
                    identity,
                    self.workbench_state.active_dataset_profile,
                    self.workbench_state
                        .active_session
                        .as_ref()
                        .and_then(|session| session.display_name.as_deref()),
                )
            })
            .unwrap_or_else(DatasetDisplayIdentity::unavailable);
        let view_label = view_label(self);
        let selection_label = selection_label(self);
        let linked_selection_label = linked_selection_label(self);
        let comparison = self.workbench_state.active_comparison.clone();
        let (axis_primary_label, axis_secondary_label) = axis_labels(self);
        let scatter_inspection = scatter_inspection_ui_state(self);
        let inspection_presentation = self.inspection_presentation.retained_content().cloned();
        let scatter_filters = scatter_filters_ui_state(self);
        let active_cohort_row_count = if self.demo_mode.is_scatter() {
            scatter_filters.as_ref().map_or(
                self.workbench_state
                    .dataset_identity
                    .as_ref()
                    .map_or(0, |identity| identity.row_count),
                |filters| filters.included_count,
            )
        } else {
            self.workbench_state
                .dataset_identity
                .as_ref()
                .map_or(0, |identity| identity.row_count)
        };
        let view_axes = view_axes_ui_state(self);
        let density_encoding = density_encoding_ui_state(self);
        let view_context = view_context_ui_state(self);
        let drilldown = match self.demo_mode {
            DemoMode::Scatter => self.scatter.selection_drilldown.clone(),
            DemoMode::Timeline => self.timeline.selection_drilldown.clone(),
        };
        let missingness = missingness_ui_state(self);
        let dataset_diff = dataset_diff_ui_state(self);
        let can_switch_to_scatter = self
            .input
            .as_ref()
            .is_none_or(|input| matches!(input, crate::cli::WorkbenchInput::Scatter { .. }));
        let can_switch_to_timeline = self
            .input
            .as_ref()
            .is_none_or(|input| matches!(input, crate::cli::WorkbenchInput::Timeline { .. }));
        let can_show_missingness = self.missingness_is_available();
        let can_show_dataset_diff = self.dataset_diff_is_available();
        let can_reset = match self.workbench_state.visible_surface {
            WorkbenchSurface::Primary => match self.demo_mode {
                DemoMode::Scatter => self.scatter.viewport.is_some(),
                DemoMode::Timeline => self.timeline.viewport.is_some(),
            },
            WorkbenchSurface::Missingness => false,
            WorkbenchSurface::DatasetDiff => false,
        };
        let can_export = match self.workbench_state.visible_surface {
            WorkbenchSurface::Primary => match self.demo_mode {
                DemoMode::Scatter => self.scatter.selection_evidence.is_some(),
                DemoMode::Timeline => self.timeline.selection_evidence.is_some(),
            },
            WorkbenchSurface::Missingness => false,
            WorkbenchSurface::DatasetDiff => false,
        };
        let can_clear_selection = match self.workbench_state.visible_surface {
            WorkbenchSurface::Primary => match self.demo_mode {
                DemoMode::Scatter => {
                    self.scatter.active_brush_selection.is_some()
                        || self.scatter.active_brush_drag.is_some()
                        || self.scatter.selection_summary.is_some()
                        || self.scatter.selection_evidence.is_some()
                        || self.scatter.selection_drilldown.is_some()
                }
                DemoMode::Timeline => {
                    self.timeline.active_brush_selection.is_some()
                        || self.timeline.active_brush_drag.is_some()
                        || self.timeline.selection_summary.is_some()
                        || self.timeline.selection_evidence.is_some()
                        || self.timeline.selection_drilldown.is_some()
                }
            },
            WorkbenchSurface::Missingness => self.missingness.selection_summary.is_some(),
            WorkbenchSurface::DatasetDiff => false,
        };

        WorkbenchUiState {
            active_view,
            visible_surface,
            dataset_identity,
            shell: self.shell,
            interaction_mode: self.interaction_mode,
            inspect_cursor_position: self.inspect_cursor_position,
            view_label,
            selection_label,
            linked_selection_label,
            comparison,
            axis_primary_label,
            axis_secondary_label,
            view_axes,
            density_encoding,
            view_context,
            export_status: self.workbench_state.export_status.clone(),
            drilldown,
            missingness,
            dataset_diff,
            can_switch_to_scatter,
            can_switch_to_timeline,
            can_show_missingness,
            can_show_dataset_diff,
            can_reset,
            can_export,
            can_clear_selection,
            density_is_refining: self.render_schedule.is_refining(),
            reduced_motion: self.visual_transition.config.reduced_motion,
            scatter_filters,
            active_cohort_row_count,
            scatter_inspection,
            inspection_presentation,
            inspection_presentation_frame: self.inspection_presentation.frame(),
        }
    }

    pub(crate) fn show_ui(
        &mut self,
        ui: &mut egui::Ui,
        pixels_per_point: f32,
        surface_width_px: u32,
        surface_height_px: u32,
    ) -> WorkbenchUiOutput {
        self.update_inspection_presentation();
        let ui_state = self.ui_state();
        show_workbench_ui(
            ui,
            &ui_state,
            pixels_per_point,
            surface_width_px,
            surface_height_px,
        )
    }

    pub(crate) fn apply_ui_actions(&mut self, actions: UiActions) {
        if let Some(mode) = actions.set_interaction_mode {
            self.set_interaction_mode(mode);
        }
        if actions.toggle_inspector {
            self.shell.toggle_inspector();
            self.request_redraw();
        }
        if actions.copy_dataset_path {
            if let Some(path) =
                self.workbench_state
                    .dataset_identity
                    .as_ref()
                    .and_then(|identity| match &identity.source {
                        rawscope_data::DatasetSource::LocalCsv { path, .. }
                        | rawscope_data::DatasetSource::LocalParquet { path, .. } => Some(path),
                        rawscope_data::DatasetSource::Synthetic { .. } => None,
                    })
            {
                self.egui_context.copy_text(path.display().to_string());
            }
        }
        if let Some(next_view) = actions.activate_view {
            let next_mode = match next_view {
                ActiveView::Scatter => DemoMode::Scatter,
                ActiveView::Timeline => DemoMode::Timeline,
            };
            self.switch_demo_mode(next_mode);
        }
        if let Some(next_surface) = actions.activate_surface {
            match next_surface {
                WorkbenchSurface::Primary => self.show_primary_surface(),
                WorkbenchSurface::Missingness => self.show_missingness_surface(),
                WorkbenchSurface::DatasetDiff => self.show_dataset_diff_surface(),
            }
        }

        if actions.reset_requested {
            match self.demo_mode {
                DemoMode::Scatter => self.reset_viewport(),
                DemoMode::Timeline => self.reset_timeline_viewport(),
            }
        }

        if let Some(transform) = actions.set_density_transform {
            self.set_density_transform(transform);
        }
        if let Some(presentation) = actions.set_scatter_density_presentation {
            self.set_scatter_density_presentation(presentation);
        }
        if let Some(mode) = actions.set_point_reveal_mode {
            self.set_point_reveal_mode(mode);
        }
        if let Some(projection) = actions.set_scatter_projection {
            if let Err(err) = self.set_scatter_projection(projection) {
                tracing::error!(error = %err, "failed to switch scatter projection");
            }
        }
        if let Some(mode) = actions.set_scatter_density_mode {
            self.set_scatter_density_mode(mode);
        }
        if let Some(config) = actions.set_relief_config {
            self.set_relief_config(config);
        }
        if let Some(reduced_motion) = actions.set_reduced_motion {
            self.set_reduced_motion(reduced_motion);
        }

        if actions.export_requested {
            match self.demo_mode {
                DemoMode::Scatter => self.export_selection_evidence(),
                DemoMode::Timeline => self.export_timeline_selection_evidence(),
            }
        }

        if actions.clear_selection_requested {
            match self.workbench_state.visible_surface {
                WorkbenchSurface::Primary => match self.demo_mode {
                    DemoMode::Scatter => self.clear_brush(),
                    DemoMode::Timeline => self.clear_timeline_brush(),
                },
                WorkbenchSurface::Missingness => {
                    self.clear_missingness_selection();
                    self.request_redraw();
                    self.update_window_title();
                }
                WorkbenchSurface::DatasetDiff => {}
            }
        }

        if let Some(action) = actions.missingness_action {
            match action {
                crate::ui_missingness::MissingnessAction::SelectCell {
                    row_bucket,
                    column_index,
                } => self.select_missingness_cell(row_bucket, column_index),
            }
        }

        if let Some(action) = actions.dataset_diff_action {
            match action {
                crate::ui_dataset_diff::DatasetDiffAction::InspectMissingnessColumn {
                    column_name,
                } => self.inspect_dataset_diff_missingness_column(&column_name),
            }
        }

        if let Some(action) = actions.filter_action {
            self.apply_scatter_filter_action(action);
        }

        if let Some(action) = actions.scatter_inspection_action {
            match action {
                crate::ui_scatter_inspection::ScatterInspectionAction::ClearPinned => {
                    self.clear_pinned_scatter_inspection()
                }
            }
        }
    }

    pub(crate) fn switch_demo_mode(&mut self, next_mode: DemoMode) {
        if self.demo_mode == next_mode {
            return;
        }
        if self.input.is_some()
            && ((next_mode.is_scatter() && self.ui_state().can_switch_to_scatter)
                || (next_mode.is_timeline() && self.ui_state().can_switch_to_timeline))
        {
            // For input-backed sessions we only allow switching to the matching bound view.
        } else if self.input.is_some() {
            return;
        }

        let Some(gpu) = self.gpu.take() else {
            self.demo_mode = next_mode;
            self.clear_dataset_diff_state();
            self.workbench_state.visible_surface = WorkbenchSurface::Primary;
            return;
        };

        self.demo_mode = next_mode;
        self.workbench_state.export_status = ExportStatus::Idle;
        self.show_primary_surface();
        let prepare_result = match self.demo_mode {
            DemoMode::Scatter => self.prepare_scatter_demo(&gpu),
            DemoMode::Timeline => self.prepare_timeline_demo(&gpu),
        };
        self.gpu = Some(gpu);

        if prepare_result.is_ok() {
            match self.demo_mode {
                DemoMode::Scatter => self.clear_brush(),
                DemoMode::Timeline => self.clear_timeline_brush(),
            }
            self.request_redraw();
            self.update_window_title();
        }
    }
}

fn view_label(app: &WorkbenchApp) -> String {
    if app.workbench_state.visible_surface == WorkbenchSurface::DatasetDiff {
        let Some(summary) = app.workbench_state.dataset_diff_summary.as_ref() else {
            return "Dataset diff unavailable".to_string();
        };
        return format!(
            "dataset diff | rows {} -> {} ({:+}) | columns {} | missingness {}",
            summary.before_row_count,
            summary.after_row_count,
            summary.row_count_delta,
            summary.columns.len(),
            summary.missingness.len()
        );
    }

    if app.workbench_state.visible_surface == WorkbenchSurface::Missingness {
        let Some(grid) = app.missingness.grid.as_ref() else {
            return "Missingness unavailable".to_string();
        };
        return format!(
            "missingness buckets {} | columns {}",
            grid.row_bucket_count, grid.column_count
        );
    }

    match app.demo_mode {
        DemoMode::Scatter => {
            let Some(viewport) = app.scatter.viewport else {
                return "Scatter view unavailable".to_string();
            };
            let Some(render_stats) = app.scatter.render_stats else {
                return "Scatter view unavailable".to_string();
            };

            format!(
                "grid {}x{} | pts {} ({}) | max {}",
                render_stats.grid_width,
                render_stats.grid_height,
                render_stats.point_count,
                app.scatter.point_count_label,
                render_stats.max_bin_count
            ) + &format!(
                " | x {:.1}..{:.1} | y {:.1}..{:.1}",
                viewport.x_range().min,
                viewport.x_range().max,
                viewport.y_range().min,
                viewport.y_range().max
            )
        }
        DemoMode::Timeline => {
            let Some(viewport) = app.timeline.viewport else {
                return "Timeline view unavailable".to_string();
            };
            let Some(render_stats) = app.timeline.render_stats else {
                return "Timeline view unavailable".to_string();
            };

            format!(
                "grid {}x{} | events {} | lanes {} | max {} | time {}..{} | full {}..{}",
                render_stats.grid_width,
                render_stats.grid_height,
                render_stats.event_count,
                render_stats.lane_count,
                render_stats.max_bin_count,
                viewport.time_range().min,
                viewport.time_range().max,
                viewport.full_time_range().min,
                viewport.full_time_range().max
            )
        }
    }
}

fn selection_label(app: &WorkbenchApp) -> String {
    if app.workbench_state.visible_surface == WorkbenchSurface::DatasetDiff {
        let Some(summary) = app.workbench_state.dataset_diff_summary.as_ref() else {
            return "Dataset diff unavailable".to_string();
        };
        return format!(
            "Rows {} -> {} ({:+})",
            summary.before_row_count, summary.after_row_count, summary.row_count_delta
        );
    }

    if app.workbench_state.visible_surface == WorkbenchSurface::Missingness {
        let Some(summary) = app.missingness.selection_summary.as_ref() else {
            return "No missingness selection".to_string();
        };
        return format!(
            "Missing {} of {} ({:.2}%)",
            summary.selected_missing_count,
            summary.selected_total_count,
            summary.selected_missing_ratio() * 100.0
        );
    }

    match app.demo_mode {
        DemoMode::Scatter => {
            if let Some(evidence) = &app.scatter.selection_evidence {
                return format!(
                    "Selection {} rows ({:.2}%)",
                    evidence.selected_row_count, evidence.selected_percentage
                );
            }
            if let Some(summary) = app.scatter.selection_summary {
                return format!(
                    "Selection {} rows ({:.2}%)",
                    summary.selected_row_count, summary.selected_percentage
                );
            }
            if app.scatter.active_brush_drag.is_some() {
                return "Brush active".to_string();
            }
            "No scatter selection".to_string()
        }
        DemoMode::Timeline => {
            if let Some(evidence) = &app.timeline.selection_evidence {
                return format!(
                    "Selection {} events ({:.2}%)",
                    evidence.selected_event_count, evidence.selected_percentage
                );
            }
            if let Some(summary) = &app.timeline.selection_summary {
                return format!(
                    "Selection {} events ({:.2}%)",
                    summary.selected_event_count, summary.selected_percentage
                );
            }
            if app.timeline.active_brush_drag.is_some() {
                return "Brush active".to_string();
            }
            "No timeline selection".to_string()
        }
    }
}

fn axis_labels(app: &WorkbenchApp) -> (String, String) {
    if app.workbench_state.visible_surface == WorkbenchSurface::DatasetDiff {
        let Some(summary) = app.workbench_state.dataset_diff_summary.as_ref() else {
            return (
                "dataset diff unavailable".to_string(),
                "dataset diff unavailable".to_string(),
            );
        };
        return (
            format!(
                "rows {} -> {} ({:+})",
                summary.before_row_count, summary.after_row_count, summary.row_count_delta
            ),
            format!(
                "schema {} | missingness {}",
                summary.columns.len(),
                summary.missingness.len()
            ),
        );
    }

    if app.workbench_state.visible_surface == WorkbenchSurface::Missingness {
        let Some(grid) = app.missingness.grid.as_ref() else {
            return (
                "missingness rows unavailable".to_string(),
                "missingness columns unavailable".to_string(),
            );
        };
        return (
            format!("row buckets 0..{}", grid.row_bucket_count),
            format!("columns {}", grid.column_count),
        );
    }

    match app.demo_mode {
        DemoMode::Scatter => {
            let Some(viewport) = app.scatter.viewport else {
                return ("x unavailable".to_string(), "y unavailable".to_string());
            };
            (
                format!(
                    "x {:.1}..{:.1}",
                    viewport.x_range().min,
                    viewport.x_range().max
                ),
                format!(
                    "y {:.1}..{:.1}",
                    viewport.y_range().min,
                    viewport.y_range().max
                ),
            )
        }
        DemoMode::Timeline => {
            let Some(viewport) = app.timeline.viewport else {
                return (
                    "time unavailable".to_string(),
                    "lanes unavailable".to_string(),
                );
            };
            (
                format!(
                    "time {}..{}",
                    viewport.time_range().min,
                    viewport.time_range().max
                ),
                format!("lanes 0..{}", viewport.lane_count()),
            )
        }
    }
}

fn linked_selection_label(app: &WorkbenchApp) -> String {
    let Some(active_selection) = app.workbench_state.active_selection.as_ref() else {
        return "Linked selection idle".to_string();
    };

    let source_view = match active_selection.visual_selection.kind() {
        VisualSelectionKind::ScatterRect => "scatter",
        VisualSelectionKind::TimelineRect => "timeline",
    };

    format!(
        "Linked {} rows from {}",
        app.active_linked_selection_count(),
        source_view
    )
}

fn missingness_ui_state(app: &WorkbenchApp) -> Option<MissingnessUiState> {
    let grid = app.missingness.grid.clone()?;
    let source_rows = match app.demo_mode {
        DemoMode::Scatter => app.scatter.source_rows.as_ref()?,
        DemoMode::Timeline => app.timeline.source_rows.as_ref()?,
    };
    let column_names = source_rows
        .column_names()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let row_bucket_labels = (0..grid.row_bucket_count)
        .map(|row_bucket| {
            let bucket_start =
                (row_bucket as usize * source_rows.rows.len()) / grid.row_bucket_count as usize;
            let bucket_end = ((row_bucket as usize + 1) * source_rows.rows.len())
                / grid.row_bucket_count as usize;
            format!("rows {bucket_start}..{bucket_end}")
        })
        .collect();

    Some(MissingnessUiState {
        grid,
        selection: app.missingness.selection,
        selection_summary: app.missingness.selection_summary.clone(),
        column_names,
        row_bucket_labels,
    })
}

fn dataset_diff_ui_state(app: &WorkbenchApp) -> Option<DatasetDiffUiState> {
    let summary = app.workbench_state.dataset_diff_summary.clone()?;
    let before_label = app
        .input
        .as_ref()
        .map(workbench_input_path_label)
        .unwrap_or_else(|| "active dataset".to_string());
    let after_label = app
        .compare_input
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "comparison dataset".to_string());

    Some(DatasetDiffUiState {
        summary,
        before_label,
        after_label,
    })
}

fn workbench_input_path_label(input: &crate::cli::WorkbenchInput) -> String {
    match input {
        crate::cli::WorkbenchInput::Scatter { path, .. }
        | crate::cli::WorkbenchInput::Timeline { path, .. } => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::ExportStatus;

    #[test]
    fn exported_status_label_includes_paths() {
        let label = ExportStatus::Exported {
            bundle_dir: "target/rawscope-exports/report-scatter-1234-1".to_string(),
        }
        .label();

        assert!(label.contains("report-scatter-1234-1"));
    }
}
