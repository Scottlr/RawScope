//! Selection drilldown panel rendering helpers.

use egui::{Grid, RichText, ScrollArea, Ui};
use rawscope_render::SelectionDrilldown;

pub(crate) fn show_selection_drilldown(ui: &mut Ui, drilldown: Option<&SelectionDrilldown>) {
    let Some(drilldown) = drilldown else {
        ui.label("No selected rows.");
        return;
    };

    ui.label(format!(
        "{} visible of {} selected{}",
        drilldown.displayed_row_count,
        drilldown.selected_row_count,
        if drilldown.rows_are_sampled {
            " (sampled)"
        } else {
            ""
        }
    ));
    ui.separator();

    if drilldown.rows.is_empty() {
        ui.label("No rows available for the current selection.");
        return;
    }

    ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Grid::new("selection_drilldown_grid")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("row_id").strong());
                    for column in &drilldown.columns {
                        ui.label(RichText::new(&column.name).strong());
                    }
                    ui.end_row();

                    for row in &drilldown.rows {
                        ui.monospace(row.row_id.0.to_string());
                        for value in &row.values {
                            ui.monospace(value);
                        }
                        ui.end_row();
                    }
                });
        });
}
