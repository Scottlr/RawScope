//! egui control panels for the workbench shell.

use egui::{Align, Button, Layout, Panel, RichText, Ui};

use crate::{
    ui::{ActiveView, WorkbenchSurface, WorkbenchUiState},
    ui_drilldown::show_selection_drilldown,
    ui_missingness::{show_missingness_summary, show_missingness_view, MissingnessAction},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct UiActions {
    pub(crate) activate_view: Option<ActiveView>,
    pub(crate) activate_surface: Option<WorkbenchSurface>,
    pub(crate) reset_requested: bool,
    pub(crate) export_requested: bool,
    pub(crate) clear_selection_requested: bool,
    pub(crate) missingness_action: Option<MissingnessAction>,
}

pub(crate) fn show_workbench_ui(ui: &mut Ui, state: &WorkbenchUiState) -> UiActions {
    let mut actions = UiActions::default();

    Panel::top("workbench_toolbar").show(ui, |ui| {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.heading("RawScope");
            ui.separator();

            let scatter_button =
                Button::new("Scatter").selected(state.active_view == ActiveView::Scatter);
            if ui
                .add_enabled(state.can_switch_to_scatter, scatter_button)
                .clicked()
            {
                actions.activate_view = Some(ActiveView::Scatter);
                actions.activate_surface = Some(WorkbenchSurface::Primary);
            }

            let timeline_button =
                Button::new("Timeline").selected(state.active_view == ActiveView::Timeline);
            if ui
                .add_enabled(state.can_switch_to_timeline, timeline_button)
                .clicked()
            {
                actions.activate_view = Some(ActiveView::Timeline);
                actions.activate_surface = Some(WorkbenchSurface::Primary);
            }

            let missingness_button = Button::new("Missingness")
                .selected(state.visible_surface == WorkbenchSurface::Missingness);
            if ui
                .add_enabled(state.can_show_missingness, missingness_button)
                .clicked()
            {
                actions.activate_surface = Some(WorkbenchSurface::Missingness);
            }

            ui.separator();
            ui.label(RichText::new(&state.dataset_label).strong());
            ui.separator();
            ui.label(&state.selection_label);
            ui.separator();
            ui.label(&state.linked_selection_label);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(state.can_export, Button::new("Export"))
                    .clicked()
                {
                    actions.export_requested = true;
                }
                if ui
                    .add_enabled(state.can_reset, Button::new("Reset"))
                    .clicked()
                {
                    actions.reset_requested = true;
                }
                if ui
                    .add_enabled(state.can_clear_selection, Button::new("Clear Selection"))
                    .clicked()
                {
                    actions.clear_selection_requested = true;
                }
            });
        });
        ui.add_space(6.0);
    });

    Panel::right("selection_drilldown_panel")
        .default_size(360.0)
        .min_size(300.0)
        .show(ui, |ui| {
            if state.visible_surface == WorkbenchSurface::Missingness {
                show_missingness_summary(ui, state.missingness.as_ref());
            } else {
                ui.heading("Selected Rows");
                ui.label(&state.view_label);
                ui.separator();
                show_selection_drilldown(ui, state.drilldown.as_ref());
            }
        });

    Panel::bottom("workbench_status_bar").show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(&state.axis_primary_label);
            ui.separator();
            ui.label(&state.axis_secondary_label);
            ui.separator();
            ui.label(state.export_status.label());
        });
        ui.add_space(4.0);
    });

    if state.visible_surface == WorkbenchSurface::Missingness {
        actions.missingness_action = state
            .missingness
            .as_ref()
            .and_then(|missingness| show_missingness_view(ui, missingness));
    }

    actions
}
