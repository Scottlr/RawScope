//! Workbench chrome, panel layout, and inspector visibility.

use egui::{Align, Layout, Panel, RichText, ScrollArea, Ui};

use crate::{
    app_interaction_mode::WorkbenchInteractionMode,
    ui::{ActiveView, ExportStatus, WorkbenchSurface, WorkbenchUiState},
    ui_comparison::show_selection_comparison,
    ui_controls::UiActions,
    ui_dataset_diff::{show_dataset_diff_summary, show_dataset_diff_view},
    ui_drilldown::show_selection_drilldown,
    ui_missingness::{show_missingness_summary, show_missingness_view},
    ui_plot_axes::show_plot_axes,
    ui_plot_surface::{allocate_plot_surface, PlotSurfaceLayout},
    ui_theme::{
        export_status_color, icon_command_button, icon_segment_button, navigation_button,
        right_rail_frame, status_badge, status_bar_frame, toolbar_frame, ACCENT, TEXT_MUTED,
    },
    ui_view_context::show_view_context,
    ui_visual_encoding::show_density_encoding,
};

const INSPECTOR_EXPANDED_WIDTH: f32 = 350.0;
const INSPECTOR_MINIMUM_WIDTH: f32 = 310.0;
const INSPECTOR_COLLAPSED_WIDTH: f32 = 60.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum InspectorState {
    Collapsed,
    #[default]
    Expanded,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WorkbenchShellState {
    pub(crate) inspector: InspectorState,
}

impl WorkbenchShellState {
    pub(crate) fn toggle_inspector(&mut self) {
        self.inspector = match self.inspector {
            InspectorState::Collapsed => InspectorState::Expanded,
            InspectorState::Expanded => InspectorState::Collapsed,
        };
    }
}

#[derive(Default)]
pub(crate) struct WorkbenchUiOutput {
    pub(crate) actions: UiActions,
    pub(crate) plot_surface: Option<PlotSurfaceLayout>,
}

pub(crate) fn show_workbench_ui(
    ui: &mut Ui,
    state: &WorkbenchUiState,
    pixels_per_point: f32,
    surface_width_px: u32,
    surface_height_px: u32,
) -> WorkbenchUiOutput {
    let mut actions = UiActions::default();

    Panel::top("workbench_toolbar")
        .frame(toolbar_frame())
        .show(ui, |ui| {
            show_primary_toolbar(ui, state, &mut actions);
            ui.add_space(4.0);
            show_session_context(ui, state, &mut actions);
        });

    show_inspector_panel(ui, state, &mut actions);

    Panel::bottom("workbench_status_bar")
        .frame(status_bar_frame())
        .show(ui, |ui| show_status_bar(ui, state));

    let plot_surface = match state.visible_surface {
        WorkbenchSurface::Missingness => {
            actions.missingness_action = state
                .missingness
                .as_ref()
                .and_then(|missingness| show_missingness_view(ui, missingness));
            None
        }
        WorkbenchSurface::DatasetDiff => {
            actions.dataset_diff_action = state
                .dataset_diff
                .as_ref()
                .and_then(|dataset_diff| show_dataset_diff_view(ui, dataset_diff));
            None
        }
        WorkbenchSurface::Primary => {
            let plot_surface =
                allocate_plot_surface(ui, pixels_per_point, surface_width_px, surface_height_px);
            if let Some(plot_surface) = plot_surface {
                show_plot_axes(ui, plot_surface.axis_layout, state.view_axes.as_ref());
            }
            plot_surface
        }
    };

    WorkbenchUiOutput {
        actions,
        plot_surface,
    }
}

fn show_primary_toolbar(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("RawScope").size(19.0).strong().color(ACCENT));
        ui.add_space(10.0);

        if ui
            .add_enabled(
                state.can_switch_to_scatter,
                navigation_button("Scatter", state.active_view == ActiveView::Scatter),
            )
            .clicked()
        {
            actions.activate_view = Some(ActiveView::Scatter);
            actions.activate_surface = Some(WorkbenchSurface::Primary);
        }
        if ui
            .add_enabled(
                state.can_switch_to_timeline,
                navigation_button("Timeline", state.active_view == ActiveView::Timeline),
            )
            .clicked()
        {
            actions.activate_view = Some(ActiveView::Timeline);
            actions.activate_surface = Some(WorkbenchSurface::Primary);
        }
        if ui
            .add_enabled(
                state.can_show_missingness,
                navigation_button(
                    "Missingness",
                    state.visible_surface == WorkbenchSurface::Missingness,
                ),
            )
            .clicked()
        {
            actions.activate_surface = Some(WorkbenchSurface::Missingness);
        }
        if state.can_show_dataset_diff
            && ui
                .add(navigation_button(
                    "Dataset Diff",
                    state.visible_surface == WorkbenchSurface::DatasetDiff,
                ))
                .clicked()
        {
            actions.activate_surface = Some(WorkbenchSurface::DatasetDiff);
        }

        ui.separator();
        show_interaction_modes(ui, state, actions);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_command_button(ui, "download", "Export evidence", state.can_export).clicked() {
                actions.export_requested = true;
            }
            if icon_command_button(ui, "rotate-ccw", "Reset view", state.can_reset).clicked() {
                actions.reset_requested = true;
            }
            if icon_command_button(ui, "eraser", "Clear selection", state.can_clear_selection)
                .clicked()
            {
                actions.clear_selection_requested = true;
            }
        });
    });
}

fn show_interaction_modes(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    let modes = [
        (WorkbenchInteractionMode::Pan, "hand", "Pan (H)"),
        (WorkbenchInteractionMode::Brush, "scan", "Brush (B)"),
        (
            WorkbenchInteractionMode::Inspect,
            "scan-search",
            "Inspect coordinates (I)",
        ),
    ];
    let previous_spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 2.0;
    for (mode, icon, tooltip) in modes {
        if icon_segment_button(ui, icon, tooltip, state.interaction_mode == mode).clicked() {
            actions.set_interaction_mode = Some(mode);
        }
    }
    ui.spacing_mut().item_spacing.x = previous_spacing;
}

fn show_session_context(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(state.dataset_identity.visible_label())
                .small()
                .strong()
                .color(TEXT_MUTED),
        )
        .on_hover_text(state.dataset_identity.details_label());
        if state.dataset_identity.full_path.is_some()
            && icon_command_button(ui, "copy", "Copy dataset path", true).clicked()
        {
            actions.copy_dataset_path = true;
        }
        status_badge(ui, &state.selection_label, state.can_clear_selection);
        status_badge(
            ui,
            &state.linked_selection_label,
            state.comparison.is_some(),
        );
    });
}

fn show_inspector_panel(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    let (default_width, minimum_width) = match state.shell.inspector {
        InspectorState::Expanded => (INSPECTOR_EXPANDED_WIDTH, INSPECTOR_MINIMUM_WIDTH),
        InspectorState::Collapsed => (INSPECTOR_COLLAPSED_WIDTH, INSPECTOR_COLLAPSED_WIDTH),
    };
    let panel_id = match state.shell.inspector {
        InspectorState::Expanded => "selection_drilldown_panel_expanded",
        InspectorState::Collapsed => "selection_drilldown_panel_collapsed",
    };
    Panel::right(panel_id)
        .default_size(default_width)
        .min_size(minimum_width)
        .max_size(if state.shell.inspector == InspectorState::Collapsed {
            INSPECTOR_COLLAPSED_WIDTH
        } else {
            f32::INFINITY
        })
        .resizable(state.shell.inspector == InspectorState::Expanded)
        .frame(right_rail_frame())
        .show(ui, |ui| {
            let (icon, tooltip) = match state.shell.inspector {
                InspectorState::Expanded => ("panel-right-close", "Collapse inspector"),
                InspectorState::Collapsed => ("panel-right-open", "Expand inspector"),
            };
            if icon_command_button(ui, icon, tooltip, true).clicked() {
                actions.toggle_inspector = true;
            }
            if state.shell.inspector == InspectorState::Expanded {
                ui.separator();
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| show_right_rail(ui, state, actions));
            }
        });
}

fn show_right_rail(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    match state.visible_surface {
        WorkbenchSurface::Missingness => {
            if show_selection_comparison(ui, state.comparison.as_ref()) {
                ui.separator();
            }
            show_missingness_summary(ui, state.missingness.as_ref());
        }
        WorkbenchSurface::DatasetDiff => show_dataset_diff_summary(ui, state.dataset_diff.as_ref()),
        WorkbenchSurface::Primary => {
            let density_response = show_density_encoding(ui, state.density_encoding.as_ref());
            actions.set_density_transform = density_response.set_transform;
            actions.set_scatter_density_presentation = density_response.set_scatter_presentation;
            if density_response.shown {
                ui.separator();
            }
            if show_view_context(ui, state.view_context.as_ref()) {
                ui.separator();
            }
            if show_selection_comparison(ui, state.comparison.as_ref()) {
                ui.separator();
            }
            ui.heading("Selected Rows");
            ui.label(RichText::new(&state.view_label).small().color(TEXT_MUTED));
            ui.separator();
            show_selection_drilldown(ui, state.drilldown.as_ref());
        }
    }
}

fn show_status_bar(ui: &mut Ui, state: &WorkbenchUiState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(&state.axis_primary_label)
                .monospace()
                .small()
                .color(TEXT_MUTED),
        );
        ui.separator();
        ui.label(
            RichText::new(format!(
                "Cohort {}",
                state.dataset_identity.row_count_label()
            ))
            .monospace()
            .small()
            .color(TEXT_MUTED),
        );
        ui.separator();
        ui.label(
            RichText::new(&state.axis_secondary_label)
                .monospace()
                .small()
                .color(TEXT_MUTED),
        );
        if let Some(position) = state.inspect_cursor_position {
            ui.separator();
            ui.label(
                RichText::new(format!("Inspect x {:.6}  y {:.6}", position.x, position.y))
                    .monospace()
                    .small()
                    .color(ACCENT),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let status_is_error = matches!(&state.export_status, ExportStatus::Failed { .. });
            let status_is_complete = matches!(&state.export_status, ExportStatus::Exported { .. });
            ui.label(
                RichText::new(state.export_status.label())
                    .small()
                    .color(export_status_color(status_is_error, status_is_complete)),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{
        InspectorState, WorkbenchShellState, INSPECTOR_COLLAPSED_WIDTH, INSPECTOR_EXPANDED_WIDTH,
    };
    use crate::ui_controls::UiActions;

    #[test]
    fn toggle_inspector_action_preserves_other_ui_actions() {
        let actions = UiActions {
            reset_requested: true,
            toggle_inspector: true,
            ..UiActions::default()
        };
        let mut shell = WorkbenchShellState::default();

        if actions.toggle_inspector {
            shell.toggle_inspector();
        }

        assert_eq!(shell.inspector, InspectorState::Collapsed);
        assert!(actions.reset_requested);
    }

    #[test]
    fn collapsed_inspector_returns_wider_plot_surface() {
        let available_width = 1_280.0;

        let expanded_plot_width = available_width - INSPECTOR_EXPANDED_WIDTH;
        let collapsed_plot_width = available_width - INSPECTOR_COLLAPSED_WIDTH;

        assert!(collapsed_plot_width > expanded_plot_width);
    }
}
