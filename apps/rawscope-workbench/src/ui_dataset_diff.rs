//! Compact dataset diff rendering for the workbench.

use std::cmp::Ordering;

use egui::{Button, CentralPanel, Grid, RichText, ScrollArea, Ui};
use rawscope_render::{DatasetDiffColumn, DatasetDiffColumnStatus, DatasetDiffMissingnessDelta};

use crate::ui::DatasetDiffUiState;

const TOP_MISSINGNESS_DELTA_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DatasetDiffAction {
    InspectMissingnessColumn { column_name: String },
}

pub(crate) fn show_dataset_diff_summary(ui: &mut Ui, state: Option<&DatasetDiffUiState>) {
    let Some(state) = state else {
        ui.label("Dataset diff unavailable for this session.");
        return;
    };

    let summary = &state.summary;
    let added_columns = columns_with_status(&summary.columns, DatasetDiffColumnStatus::Added);
    let removed_columns = columns_with_status(&summary.columns, DatasetDiffColumnStatus::Removed);
    let type_changed_columns =
        columns_with_status(&summary.columns, DatasetDiffColumnStatus::TypeChanged);
    let unchanged_columns =
        columns_with_status(&summary.columns, DatasetDiffColumnStatus::Unchanged);

    ui.heading("Dataset Diff");
    ui.label(format!("Before: {}", state.before_label));
    ui.label(format!("After: {}", state.after_label));
    ui.separator();
    ui.label(format!("Rows before: {}", summary.before_row_count));
    ui.label(format!("Rows after: {}", summary.after_row_count));
    ui.label(format!("Row delta: {:+}", summary.row_count_delta));
    ui.separator();
    ui.label(RichText::new("Schema").strong());
    ui.label(format!("Added: {}", format_column_list(&added_columns)));
    ui.label(format!("Removed: {}", format_column_list(&removed_columns)));
    ui.label(format!(
        "Type changed: {}",
        format_column_list(&type_changed_columns)
    ));
    ui.label(format!("Unchanged: {}", unchanged_columns.len()));
}

pub(crate) fn show_dataset_diff_view(
    ui: &mut Ui,
    state: &DatasetDiffUiState,
) -> Option<DatasetDiffAction> {
    let mut next_action = None;
    let top_missingness_deltas = top_missingness_deltas(&state.summary.missingness);

    CentralPanel::default().show(ui, |ui| {
        ui.heading("Dataset Diff");
        ui.label(format!("{} -> {}", state.before_label, state.after_label));
        ui.add_space(8.0);
        ui.label(format!(
            "Shared columns with the largest missingness shifts (top {}).",
            top_missingness_deltas.len()
        ));
        ui.add_space(8.0);

        if top_missingness_deltas.is_empty() {
            ui.label("No shared-column missingness deltas are available.");
            return;
        }

        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("dataset_diff_missingness_grid")
                .striped(true)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("column").strong());
                    ui.label(RichText::new("before").strong());
                    ui.label(RichText::new("after").strong());
                    ui.label(RichText::new("delta").strong());
                    ui.end_row();

                    for delta in top_missingness_deltas {
                        let inspect_button =
                            Button::new(delta.column_name.as_str()).selected(false);
                        if ui
                            .add_sized([160.0, 24.0], inspect_button)
                            .on_hover_text("Inspect this column in the active missingness surface.")
                            .clicked()
                        {
                            next_action = Some(DatasetDiffAction::InspectMissingnessColumn {
                                column_name: delta.column_name.clone(),
                            });
                        }

                        ui.label(format_percentage(delta.before_missing_ratio));
                        ui.label(format_percentage(delta.after_missing_ratio));
                        ui.label(format!("{:+.1}pp", delta.delta_percentage_points));
                        ui.end_row();
                    }
                });
        });
    });

    next_action
}

fn columns_with_status(
    columns: &[DatasetDiffColumn],
    status: DatasetDiffColumnStatus,
) -> Vec<&DatasetDiffColumn> {
    columns
        .iter()
        .filter(|column| column.status == status)
        .collect()
}

fn format_column_list(columns: &[&DatasetDiffColumn]) -> String {
    if columns.is_empty() {
        return "none".to_string();
    }

    columns
        .iter()
        .map(|column| format_column(column))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_column(column: &DatasetDiffColumn) -> String {
    match (column.before_type, column.after_type) {
        (Some(before_type), Some(after_type))
            if column.status == DatasetDiffColumnStatus::TypeChanged =>
        {
            format!("{} ({} -> {})", column.name, before_type, after_type)
        }
        (Some(before_type), _) => format!("{} ({})", column.name, before_type),
        (_, Some(after_type)) => format!("{} ({})", column.name, after_type),
        (None, None) => column.name.clone(),
    }
}

fn top_missingness_deltas(
    deltas: &[DatasetDiffMissingnessDelta],
) -> Vec<&DatasetDiffMissingnessDelta> {
    let mut top_deltas = deltas.iter().collect::<Vec<_>>();
    top_deltas.sort_by(|left, right| {
        let left_delta = left.delta_percentage_points.abs();
        let right_delta = right.delta_percentage_points.abs();
        right_delta
            .partial_cmp(&left_delta)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.column_name.cmp(&right.column_name))
    });
    top_deltas.truncate(TOP_MISSINGNESS_DELTA_LIMIT);
    top_deltas
}

fn format_percentage(ratio: f32) -> String {
    format!("{:.1}%", ratio * 100.0)
}
