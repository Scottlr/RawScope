//! Fixed navigation/tool rail for the analytical workbench shell.

use egui::{Panel, Ui};

use crate::{
    ui::{ActiveView, WorkbenchSurface, WorkbenchUiState},
    ui_controls::UiActions,
    ui_shell_layout::{ResponsiveShellProjection, ShellRegionState},
    ui_theme::{
        activity_rail_frame, icon_command_button, icon_segment_button, navigation_button,
        CHROME_METRICS,
    },
};

pub(crate) fn show_activity_rail(
    ui: &mut Ui,
    state: &WorkbenchUiState,
    actions: &mut UiActions,
    responsive: ResponsiveShellProjection,
) {
    let expanded = responsive.activity_rail == ShellRegionState::Expanded;
    let width = if expanded {
        CHROME_METRICS.activity_rail_width_points
    } else {
        CHROME_METRICS.activity_rail_collapsed_width_points
    };
    Panel::left(if expanded {
        "workbench_activity_rail_expanded"
    } else {
        "workbench_activity_rail_collapsed"
    })
    .default_size(width)
    .min_size(width)
    .max_size(width)
    .frame(activity_rail_frame())
    .show(ui, |ui| {
        let mut activate = |view: Option<ActiveView>, surface: WorkbenchSurface| {
            actions.activate_view = view;
            actions.activate_surface = Some(surface);
        };
        if expanded {
            if ui
                .add_enabled(
                    state.can_switch_to_scatter,
                    navigation_button("Scatter", state.active_view == ActiveView::Scatter),
                )
                .clicked()
            {
                activate(Some(ActiveView::Scatter), WorkbenchSurface::Primary);
            }
            if ui
                .add_enabled(
                    state.can_switch_to_timeline,
                    navigation_button("Timeline", state.active_view == ActiveView::Timeline),
                )
                .clicked()
            {
                activate(Some(ActiveView::Timeline), WorkbenchSurface::Primary);
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
                activate(None, WorkbenchSurface::Missingness);
            }
            if state.can_show_dataset_diff
                && ui
                    .add(navigation_button(
                        "Dataset Diff",
                        state.visible_surface == WorkbenchSurface::DatasetDiff,
                    ))
                    .clicked()
            {
                activate(None, WorkbenchSurface::DatasetDiff);
            }
        } else {
            if icon_segment_button(
                ui,
                "chart-scatter",
                "Scatter workspace",
                state.active_view == ActiveView::Scatter
                    && state.visible_surface == WorkbenchSurface::Primary,
            )
            .clicked()
            {
                activate(Some(ActiveView::Scatter), WorkbenchSurface::Primary);
            }
            if icon_segment_button(
                ui,
                "chart-no-axes-combined",
                "Timeline workspace",
                state.active_view == ActiveView::Timeline
                    && state.visible_surface == WorkbenchSurface::Primary,
            )
            .clicked()
            {
                activate(Some(ActiveView::Timeline), WorkbenchSurface::Primary);
            }
            if icon_segment_button(
                ui,
                "table-properties",
                "Missingness workspace",
                state.visible_surface == WorkbenchSurface::Missingness,
            )
            .clicked()
            {
                activate(None, WorkbenchSurface::Missingness);
            }
            if state.can_show_dataset_diff
                && icon_segment_button(
                    ui,
                    "columns-3",
                    "Dataset diff workspace",
                    state.visible_surface == WorkbenchSurface::DatasetDiff,
                )
                .clicked()
            {
                activate(None, WorkbenchSurface::DatasetDiff);
            }
        }
        ui.add_space(8.0);
        if icon_command_button(
            ui,
            if expanded {
                "panel-left-close"
            } else {
                "panel-left-open"
            },
            if expanded {
                "Collapse activity rail"
            } else {
                "Expand activity rail"
            },
            state.shell.activity_rail == responsive.activity_rail,
        )
        .clicked()
        {
            actions.toggle_activity_rail = true;
        }
    });
}
