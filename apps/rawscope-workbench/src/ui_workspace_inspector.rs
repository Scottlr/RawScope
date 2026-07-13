//! Properties, evidence, and inspection composition for the shell inspector.

use egui::{RichText, ScrollArea, Ui};

use crate::{
    ui::{WorkbenchSurface, WorkbenchUiState},
    ui_comparison::show_selection_comparison,
    ui_controls::UiActions,
    ui_dataset_diff::show_dataset_diff_summary,
    ui_drilldown::show_selection_drilldown,
    ui_filters::show_filters,
    ui_missingness::show_missingness_summary,
    ui_pinned_inspection::show_pinned_scatter_inspection,
    ui_shell_layout::{ResponsiveShellProjection, ShellRegionState},
    ui_theme::{icon_command_button, right_rail_frame, CHROME_METRICS, TEXT_MUTED},
    ui_view_context::show_view_context,
    ui_visual_encoding::show_density_encoding,
};

pub(crate) fn show_workspace_inspector(
    ui: &mut Ui,
    state: &WorkbenchUiState,
    actions: &mut UiActions,
    responsive: ResponsiveShellProjection,
) {
    let (default_width, minimum_width) = match responsive.inspector {
        ShellRegionState::Expanded => (
            CHROME_METRICS.inspector_width_points,
            CHROME_METRICS.inspector_width_points - 40.0,
        ),
        ShellRegionState::Collapsed => (
            CHROME_METRICS.activity_rail_collapsed_width_points,
            CHROME_METRICS.activity_rail_collapsed_width_points,
        ),
    };
    let panel_id = match responsive.inspector {
        ShellRegionState::Expanded => "selection_drilldown_panel_expanded",
        ShellRegionState::Collapsed => "selection_drilldown_panel_collapsed",
    };
    egui::Panel::right(panel_id)
        .default_size(default_width)
        .min_size(minimum_width)
        .max_size(if responsive.inspector == ShellRegionState::Collapsed {
            CHROME_METRICS.activity_rail_collapsed_width_points
        } else {
            f32::INFINITY
        })
        .resizable(responsive.inspector == ShellRegionState::Expanded)
        .frame(right_rail_frame())
        .show(ui, |ui| {
            let (icon, tooltip) = match responsive.inspector {
                ShellRegionState::Expanded => ("panel-right-close", "Collapse inspector"),
                ShellRegionState::Collapsed => ("panel-right-open", "Expand inspector"),
            };
            if icon_command_button(
                ui,
                icon,
                tooltip,
                state.shell.inspector == responsive.inspector,
            )
            .clicked()
            {
                actions.toggle_inspector = true;
            }
            if responsive.inspector == ShellRegionState::Expanded {
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
            actions.set_point_reveal_mode = density_response.set_point_reveal_mode;
            actions.set_scatter_projection = density_response.set_scatter_projection;
            actions.set_scatter_density_mode = density_response.set_scatter_density_mode;
            actions.set_relief_config = density_response.set_relief_config;
            if density_response.shown {
                ui.separator();
            }
            actions.filter_action = show_filters(ui, state.scatter_filters.as_ref());
            if state.scatter_filters.is_some() {
                ui.separator();
            }
            actions.scatter_inspection_action =
                show_pinned_scatter_inspection(ui, state.scatter_inspection.as_ref());
            if state
                .scatter_inspection
                .as_ref()
                .is_some_and(|inspection| inspection.pinned.is_some())
            {
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
