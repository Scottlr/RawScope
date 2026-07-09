//! Missingness UI rendering helpers for the first CPU-backed slice.

use egui::{Button, CentralPanel, Color32, Grid, RichText, ScrollArea, Ui};

use crate::ui::MissingnessUiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MissingnessAction {
    SelectCell { row_bucket: u32, column_index: u32 },
}

pub(crate) fn show_missingness_view(
    ui: &mut Ui,
    state: &MissingnessUiState,
) -> Option<MissingnessAction> {
    let mut next_action = None;

    CentralPanel::default().show(ui, |ui| {
        ui.heading("Missingness");
        ui.label(format!(
            "{} row buckets across {} columns",
            state.grid.row_bucket_count, state.grid.column_count
        ));
        ui.add_space(8.0);

        ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Grid::new("missingness_grid")
                    .striped(true)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("row bucket").strong());
                        for column_name in &state.column_names {
                            ui.label(RichText::new(column_name).strong());
                        }
                        ui.end_row();

                        for (row_bucket_index, row_bucket_label) in
                            state.row_bucket_labels.iter().enumerate()
                        {
                            let row_bucket = u32::try_from(row_bucket_index).unwrap_or(u32::MAX);
                            ui.monospace(row_bucket_label);

                            for column_index in 0..state.grid.column_count {
                                let Some(cell) = state.grid.cell(row_bucket, column_index) else {
                                    ui.label("-");
                                    continue;
                                };

                                let cell_is_selected = state.selection.is_some_and(|selection| {
                                    selection.row_bucket_start == row_bucket
                                        && selection.row_bucket_end_exclusive == row_bucket + 1
                                        && selection.column_start == column_index
                                        && selection.column_end_exclusive == column_index + 1
                                });
                                let button_fill = heatmap_fill(*cell);
                                let button = Button::new(format!(
                                    "{}/{}",
                                    cell.missing_count, cell.total_count
                                ))
                                .selected(cell_is_selected)
                                .fill(button_fill);
                                if ui
                                    .add_sized([72.0, 24.0], button)
                                    .on_hover_text(format!(
                                        "{} missing of {} rows ({:.1}%)",
                                        cell.missing_count,
                                        cell.total_count,
                                        cell.missing_ratio() * 100.0
                                    ))
                                    .clicked()
                                {
                                    next_action = Some(MissingnessAction::SelectCell {
                                        row_bucket,
                                        column_index,
                                    });
                                }
                            }

                            ui.end_row();
                        }
                    });
            });
    });

    next_action
}

pub(crate) fn show_missingness_summary(ui: &mut Ui, state: Option<&MissingnessUiState>) {
    let Some(state) = state else {
        ui.label("Missingness unavailable for this dataset.");
        return;
    };

    ui.heading("Missingness Selection");
    let Some(summary) = state.selection_summary.as_ref() else {
        ui.label("Select a heatmap cell to inspect missing rows.");
        return;
    };

    ui.label(format!(
        "{} missing of {} values ({:.1}%)",
        summary.selected_missing_count,
        summary.selected_total_count,
        summary.selected_missing_ratio() * 100.0
    ));
    ui.separator();
    ui.label(format!("Columns: {}", summary.column_names.join(", ")));
    ui.label(format!(
        "Rows with missing values: {}",
        summary.selected_row_ids.len()
    ));

    if summary.selected_row_ids.is_empty() {
        return;
    }

    ui.separator();
    ui.label(RichText::new("Row IDs").strong());
    ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        for row_id in &summary.selected_row_ids {
            ui.monospace(row_id.0.to_string());
        }
    });
}

fn heatmap_fill(cell: rawscope_render::MissingnessCell) -> Color32 {
    let missing_ratio = cell.missing_ratio().clamp(0.0, 1.0);
    let red = 48 + (missing_ratio * 160.0) as u8;
    let green = 56 + ((1.0 - missing_ratio) * 80.0) as u8;
    let blue = 68;
    Color32::from_rgb(red, green, blue)
}
