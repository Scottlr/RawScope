//! Workbench chrome, panel layout, and inspector visibility.

use egui::{Align, Layout, Panel, RichText, Ui};

use crate::{
    app_interaction_mode::WorkbenchInteractionMode,
    ui::{ActiveView, ExportStatus, WorkbenchSurface, WorkbenchUiState},
    ui_controls::UiActions,
    ui_dataset_diff::show_dataset_diff_view,
    ui_dataset_identity::format_row_count,
    ui_inspection_tooltip::show_inspection_tooltip,
    ui_missingness::show_missingness_view,
    ui_plot_axes::{show_plot_axes, show_scatter_marginals_in_gutters},
    ui_plot_surface::{allocate_plot_surface, PlotSurfaceLayout},
    ui_shell_layout::{project_responsive_shell, ShellRegionState},
    ui_theme::{
        export_status_color, icon_command_button, icon_segment_button, status_badge,
        status_bar_frame, toolbar_frame, ACCENT, TEXT_MUTED,
    },
    ui_workspace_inspector::show_workspace_inspector,
};

pub(crate) use crate::ui_shell_layout::ShellRegionState as InspectorState;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WorkbenchShellState {
    pub(crate) activity_rail: ShellRegionState,
    pub(crate) inspector: InspectorState,
}

impl WorkbenchShellState {
    pub(crate) fn toggle_inspector(&mut self) {
        self.inspector = match self.inspector {
            InspectorState::Collapsed => InspectorState::Expanded,
            InspectorState::Expanded => InspectorState::Collapsed,
        };
    }

    pub(crate) fn toggle_activity_rail(&mut self) {
        self.activity_rail = match self.activity_rail {
            ShellRegionState::Collapsed => ShellRegionState::Expanded,
            ShellRegionState::Expanded => ShellRegionState::Collapsed,
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
    let available_size = ui.available_rect_before_wrap().size();
    let responsive = project_responsive_shell(
        available_size.x,
        available_size.y,
        state.shell.activity_rail,
        state.shell.inspector,
    );

    Panel::top("workbench_toolbar")
        .frame(toolbar_frame())
        .show(ui, |ui| {
            show_primary_toolbar(ui, state, &mut actions);
            ui.add_space(4.0);
            if responsive.show_secondary_text {
                show_session_context(ui, state, &mut actions);
            } else {
                show_compact_session_context(ui, state, &mut actions);
            }
        });

    crate::ui_activity_rail::show_activity_rail(ui, state, &mut actions, responsive);
    show_workspace_inspector(ui, state, &mut actions, responsive);

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
                if state.active_view == crate::ui::ActiveView::Scatter {
                    let summary = state
                        .view_context
                        .as_ref()
                        .and_then(|context| context.scatter_marginals.as_ref());
                    show_scatter_marginals_in_gutters(
                        ui,
                        plot_surface.axis_layout,
                        summary,
                        state.density_is_refining,
                    );
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("Viewport unavailable at this window size").color(TEXT_MUTED),
                    );
                });
            }
            plot_surface
        }
    };
    show_inspection_tooltip(
        ui.ctx(),
        state.inspection_presentation.as_ref(),
        state.inspection_presentation_frame,
    );

    WorkbenchUiOutput {
        actions,
        plot_surface,
    }
}

fn show_primary_toolbar(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("RawScope").size(19.0).strong().color(ACCENT));
        ui.add_space(10.0);
        ui.label(
            RichText::new(match state.visible_surface {
                WorkbenchSurface::Primary => match state.active_view {
                    ActiveView::Scatter => "Scatter workspace",
                    ActiveView::Timeline => "Timeline workspace",
                },
                WorkbenchSurface::Missingness => "Missingness workspace",
                WorkbenchSurface::DatasetDiff => "Dataset diff workspace",
            })
            .small()
            .color(TEXT_MUTED),
        );
        ui.separator();
        show_interaction_modes(ui, state, actions);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.menu_button("Settings", |ui| {
                let mut reduced_motion = state.reduced_motion;
                if ui.checkbox(&mut reduced_motion, "Reduce motion").changed() {
                    actions.set_reduced_motion = Some(reduced_motion);
                }
            });
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

fn show_compact_session_context(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    ui.horizontal(|ui| {
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
    });
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
                format_row_count(state.active_cohort_row_count)
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
            if state.density_is_refining {
                ui.label(RichText::new("Refining").small().color(ACCENT));
                return;
            }
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
    use super::{InspectorState, WorkbenchShellState};
    use crate::app_interaction_mode::WorkbenchInteractionMode;
    use crate::ui::{ActiveView, WorkbenchSurface};
    use crate::ui_controls::UiActions;
    use crate::ui_theme::CHROME_METRICS;

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

        let expanded_plot_width = available_width - CHROME_METRICS.inspector_width_points;
        let collapsed_plot_width =
            available_width - CHROME_METRICS.activity_rail_collapsed_width_points;

        assert!(collapsed_plot_width > expanded_plot_width);
    }

    #[test]
    fn toggle_activity_rail_action_preserves_user_preference() {
        let mut shell = WorkbenchShellState::default();
        assert_eq!(shell.activity_rail, InspectorState::Expanded);

        shell.toggle_activity_rail();

        assert_eq!(shell.activity_rail, InspectorState::Collapsed);
        assert_eq!(shell.inspector, InspectorState::Expanded);
    }

    #[test]
    fn shell_actions_project_to_existing_typed_commands() {
        let actions = UiActions {
            activate_view: Some(ActiveView::Timeline),
            activate_surface: Some(WorkbenchSurface::Primary),
            set_interaction_mode: Some(WorkbenchInteractionMode::Inspect),
            toggle_activity_rail: true,
            ..UiActions::default()
        };

        assert_eq!(actions.activate_view, Some(ActiveView::Timeline));
        assert_eq!(actions.activate_surface, Some(WorkbenchSurface::Primary));
        assert_eq!(
            actions.set_interaction_mode,
            Some(WorkbenchInteractionMode::Inspect)
        );
        assert!(actions.toggle_activity_rail);
    }
}
