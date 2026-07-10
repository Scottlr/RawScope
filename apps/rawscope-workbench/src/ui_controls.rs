//! egui control panels for the workbench shell.

use egui::{Align, Layout, Panel, RichText, ScrollArea, Ui};
use rawscope_render::{DensityTransform, ScatterDensityPresentation};

use crate::{
    ui::{ActiveView, ExportStatus, WorkbenchSurface, WorkbenchUiState},
    ui_comparison::show_selection_comparison,
    ui_dataset_diff::{show_dataset_diff_summary, show_dataset_diff_view, DatasetDiffAction},
    ui_drilldown::show_selection_drilldown,
    ui_missingness::{show_missingness_summary, show_missingness_view, MissingnessAction},
    ui_plot_axes::show_plot_axes,
    ui_plot_surface::{allocate_plot_surface, PlotSurfaceLayout},
    ui_theme::{
        command_button, export_status_color, navigation_button, right_rail_frame, status_badge,
        status_bar_frame, toolbar_frame, ACCENT, TEXT_MUTED,
    },
    ui_view_context::show_view_context,
    ui_visual_encoding::show_density_encoding,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UiActions {
    pub(crate) activate_view: Option<ActiveView>,
    pub(crate) activate_surface: Option<WorkbenchSurface>,
    pub(crate) reset_requested: bool,
    pub(crate) export_requested: bool,
    pub(crate) clear_selection_requested: bool,
    pub(crate) set_density_transform: Option<DensityTransform>,
    pub(crate) set_scatter_density_presentation: Option<ScatterDensityPresentation>,
    pub(crate) missingness_action: Option<MissingnessAction>,
    pub(crate) dataset_diff_action: Option<DatasetDiffAction>,
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
            show_session_context(ui, state);
        });

    Panel::right("selection_drilldown_panel")
        .default_size(350.0)
        .min_size(310.0)
        .frame(right_rail_frame())
        .show(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| show_right_rail(ui, state, &mut actions));
        });

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

        let scatter_button = navigation_button("Scatter", state.active_view == ActiveView::Scatter);
        if ui
            .add_enabled(state.can_switch_to_scatter, scatter_button)
            .clicked()
        {
            actions.activate_view = Some(ActiveView::Scatter);
            actions.activate_surface = Some(WorkbenchSurface::Primary);
        }

        let timeline_button =
            navigation_button("Timeline", state.active_view == ActiveView::Timeline);
        if ui
            .add_enabled(state.can_switch_to_timeline, timeline_button)
            .clicked()
        {
            actions.activate_view = Some(ActiveView::Timeline);
            actions.activate_surface = Some(WorkbenchSurface::Primary);
        }

        let missingness_button = navigation_button(
            "Missingness",
            state.visible_surface == WorkbenchSurface::Missingness,
        );
        if ui
            .add_enabled(state.can_show_missingness, missingness_button)
            .clicked()
        {
            actions.activate_surface = Some(WorkbenchSurface::Missingness);
        }

        if state.can_show_dataset_diff {
            let dataset_diff_button = navigation_button(
                "Dataset Diff",
                state.visible_surface == WorkbenchSurface::DatasetDiff,
            );
            if ui.add(dataset_diff_button).clicked() {
                actions.activate_surface = Some(WorkbenchSurface::DatasetDiff);
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_enabled(state.can_export, command_button("Export"))
                .clicked()
            {
                actions.export_requested = true;
            }
            if ui
                .add_enabled(state.can_reset, command_button("Reset"))
                .clicked()
            {
                actions.reset_requested = true;
            }
            if ui
                .add_enabled(state.can_clear_selection, command_button("Clear selection"))
                .clicked()
            {
                actions.clear_selection_requested = true;
            }
        });
    });
}

fn show_session_context(ui: &mut Ui, state: &WorkbenchUiState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(&state.dataset_label)
                .small()
                .strong()
                .color(TEXT_MUTED),
        )
        .on_hover_text(&state.dataset_label);
        status_badge(ui, &state.selection_label, state.can_clear_selection);
        status_badge(
            ui,
            &state.linked_selection_label,
            state.comparison.is_some(),
        );
    });
}

fn show_right_rail(ui: &mut Ui, state: &WorkbenchUiState, actions: &mut UiActions) {
    match state.visible_surface {
        WorkbenchSurface::Missingness => {
            let comparison_shown = show_selection_comparison(ui, state.comparison.as_ref());
            if comparison_shown {
                ui.separator();
            }
            show_missingness_summary(ui, state.missingness.as_ref());
        }
        WorkbenchSurface::DatasetDiff => {
            show_dataset_diff_summary(ui, state.dataset_diff.as_ref());
        }
        WorkbenchSurface::Primary => {
            let density_response = show_density_encoding(ui, state.density_encoding.as_ref());
            actions.set_density_transform = density_response.set_transform;
            actions.set_scatter_density_presentation = density_response.set_scatter_presentation;
            if density_response.shown {
                ui.separator();
            }
            let context_shown = show_view_context(ui, state.view_context.as_ref());
            if context_shown {
                ui.separator();
            }
            let comparison_shown = show_selection_comparison(ui, state.comparison.as_ref());
            if comparison_shown {
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
            RichText::new(&state.axis_secondary_label)
                .monospace()
                .small()
                .color(TEXT_MUTED),
        );
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
