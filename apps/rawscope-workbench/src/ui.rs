//! Visible workbench UI state and formatting helpers.

use egui::ViewportId;
use egui_wgpu::RendererOptions;
use rawscope_core::VisualSelectionKind;
use rawscope_data::{DatasetFieldRole, DatasetSource};
use rawscope_render::{
    MissingnessGrid, MissingnessSelection, MissingnessSelectionSummary, SelectionDrilldown,
};

use crate::{
    app::WorkbenchApp,
    demo::DemoMode,
    ui_controls::{show_workbench_ui, UiActions},
};

/// Active visual workbench view shown in the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveView {
    Scatter,
    Timeline,
}

/// Visible analyst surface shown above the current dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkbenchSurface {
    #[default]
    Primary,
    Missingness,
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
    pub(crate) dataset_label: String,
    pub(crate) view_label: String,
    pub(crate) selection_label: String,
    pub(crate) linked_selection_label: String,
    pub(crate) axis_primary_label: String,
    pub(crate) axis_secondary_label: String,
    pub(crate) export_status: ExportStatus,
    pub(crate) drilldown: Option<SelectionDrilldown>,
    pub(crate) missingness: Option<MissingnessUiState>,
    pub(crate) can_switch_to_scatter: bool,
    pub(crate) can_switch_to_timeline: bool,
    pub(crate) can_show_missingness: bool,
    pub(crate) can_reset: bool,
    pub(crate) can_export: bool,
    pub(crate) can_clear_selection: bool,
}

impl WorkbenchUiState {
    pub(crate) fn window_title(&self) -> String {
        let drilldown_suffix = self
            .drilldown
            .as_ref()
            .map(|drilldown| {
                format!(
                    " | drilldown {}/{}{}",
                    drilldown.displayed_row_count,
                    drilldown.selected_row_count,
                    if drilldown.rows_are_sampled {
                        " sampled"
                    } else {
                        ""
                    }
                )
            })
            .unwrap_or_default();

        format!(
            "RawScope | {} | {} | {}{}",
            self.dataset_label, self.view_label, self.selection_label, drilldown_suffix
        )
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
        let visible_surface = self.visible_surface;
        let dataset_label = dataset_label(self);
        let view_label = view_label(self);
        let selection_label = selection_label(self);
        let linked_selection_label = linked_selection_label(self);
        let (axis_primary_label, axis_secondary_label) = axis_labels(self);
        let drilldown = match self.demo_mode {
            DemoMode::Scatter => self.scatter.selection_drilldown.clone(),
            DemoMode::Timeline => self.timeline.selection_drilldown.clone(),
        };
        let missingness = missingness_ui_state(self);
        let can_switch_to_scatter = self
            .input
            .as_ref()
            .is_none_or(|input| matches!(input, crate::cli::WorkbenchInput::Scatter { .. }));
        let can_switch_to_timeline = self
            .input
            .as_ref()
            .is_none_or(|input| matches!(input, crate::cli::WorkbenchInput::Timeline { .. }));
        let can_show_missingness = self.missingness_is_available();
        let can_reset = match self.visible_surface {
            WorkbenchSurface::Primary => match self.demo_mode {
                DemoMode::Scatter => self.scatter.viewport.is_some(),
                DemoMode::Timeline => self.timeline.viewport.is_some(),
            },
            WorkbenchSurface::Missingness => false,
        };
        let can_export = match self.visible_surface {
            WorkbenchSurface::Primary => match self.demo_mode {
                DemoMode::Scatter => self.scatter.selection_evidence.is_some(),
                DemoMode::Timeline => self.timeline.selection_evidence.is_some(),
            },
            WorkbenchSurface::Missingness => false,
        };
        let can_clear_selection = match self.visible_surface {
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
        };

        WorkbenchUiState {
            active_view,
            visible_surface,
            dataset_label,
            view_label,
            selection_label,
            linked_selection_label,
            axis_primary_label,
            axis_secondary_label,
            export_status: self.export_status.clone(),
            drilldown,
            missingness,
            can_switch_to_scatter,
            can_switch_to_timeline,
            can_show_missingness,
            can_reset,
            can_export,
            can_clear_selection,
        }
    }

    pub(crate) fn show_ui(&mut self, ui: &mut egui::Ui) -> UiActions {
        let ui_state = self.ui_state();
        show_workbench_ui(ui, &ui_state)
    }

    pub(crate) fn apply_ui_actions(&mut self, actions: UiActions) {
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
            }
        }

        if actions.reset_requested {
            match self.demo_mode {
                DemoMode::Scatter => self.reset_viewport(),
                DemoMode::Timeline => self.reset_timeline_viewport(),
            }
        }

        if actions.export_requested {
            match self.demo_mode {
                DemoMode::Scatter => self.export_selection_evidence(),
                DemoMode::Timeline => self.export_timeline_selection_evidence(),
            }
        }

        if actions.clear_selection_requested {
            match self.visible_surface {
                WorkbenchSurface::Primary => match self.demo_mode {
                    DemoMode::Scatter => self.clear_brush(),
                    DemoMode::Timeline => self.clear_timeline_brush(),
                },
                WorkbenchSurface::Missingness => {
                    self.clear_missingness_selection();
                    self.request_redraw();
                    self.update_window_title();
                }
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
            return;
        };

        self.demo_mode = next_mode;
        self.export_status = ExportStatus::Idle;
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

fn dataset_label(app: &WorkbenchApp) -> String {
    let Some(identity) = app.dataset_identity.as_ref() else {
        return "Dataset unavailable".to_string();
    };

    let bindings = identity
        .field_bindings
        .iter()
        .map(|binding| {
            let role = match binding.role {
                DatasetFieldRole::X => "x",
                DatasetFieldRole::Y => "y",
                DatasetFieldRole::Time => "time",
                DatasetFieldRole::Lane => "lane",
            };
            format!("{role}={}", binding.column_name)
        })
        .collect::<Vec<_>>()
        .join(", ");

    match &identity.source {
        DatasetSource::Synthetic { seed, generator } => {
            format!(
                "Synthetic {generator} | seed {seed} | {} rows | {bindings}",
                identity.row_count
            )
        }
        DatasetSource::LocalCsv { path, limit } => format!(
            "CSV {} | {} rows{} | {bindings}",
            path.display(),
            identity.row_count,
            limit
                .map(|value| format!(" (limit {value})"))
                .unwrap_or_default()
        ),
        DatasetSource::LocalParquet { path, limit } => format!(
            "Parquet {} | {} rows{} | {bindings}",
            path.display(),
            identity.row_count,
            limit
                .map(|value| format!(" (limit {value})"))
                .unwrap_or_default()
        ),
    }
}

fn view_label(app: &WorkbenchApp) -> String {
    if app.visible_surface == WorkbenchSurface::Missingness {
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
    if app.visible_surface == WorkbenchSurface::Missingness {
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
    if app.visible_surface == WorkbenchSurface::Missingness {
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
    let Some(active_selection) = app.active_selection.as_ref() else {
        return "Linked selection idle".to_string();
    };

    let source_view = match active_selection.visual_selection.kind {
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

#[cfg(test)]
mod tests {
    use super::{ActiveView, ExportStatus, WorkbenchSurface, WorkbenchUiState};

    #[test]
    fn exported_status_label_includes_paths() {
        let label = ExportStatus::Exported {
            bundle_dir: "target/rawscope-exports/report-scatter-1234-1".to_string(),
        }
        .label();

        assert!(label.contains("report-scatter-1234-1"));
    }

    #[test]
    fn window_title_includes_dataset_view_and_selection() {
        let state = WorkbenchUiState {
            active_view: ActiveView::Scatter,
            visible_surface: WorkbenchSurface::Primary,
            dataset_label: "Synthetic scatter | 20000 rows".to_string(),
            view_label: "grid 256x256 | x 0.0..100.0 | y 0.0..100.0".to_string(),
            selection_label: "Selection 42 rows (0.21%)".to_string(),
            linked_selection_label: "Linked 42 rows from scatter".to_string(),
            axis_primary_label: "x 0.0..100.0".to_string(),
            axis_secondary_label: "y 0.0..100.0".to_string(),
            export_status: ExportStatus::Idle,
            drilldown: None,
            missingness: None,
            can_switch_to_scatter: true,
            can_switch_to_timeline: true,
            can_show_missingness: false,
            can_reset: true,
            can_export: true,
            can_clear_selection: true,
        };

        let title = state.window_title();

        assert!(title.contains("Synthetic scatter"));
        assert!(title.contains("grid 256x256"));
        assert!(title.contains("Selection 42 rows"));
    }
}
