//! Bounded tooltip and pinned content for scatter-density inspection.

use egui::{vec2, Area, Context, Frame, Id, Order, RichText, Ui};
use rawscope_data::dataset_profile;
use rawscope_render::{
    difference_inspection, DifferenceInspection, ScatterDensityMode, ScatterInspectionHit,
};

use crate::{
    app::WorkbenchApp,
    app_scatter_inspection::PinnedScatterInspection,
    ui::WorkbenchSurface,
    ui_theme::{ACCENT, TEXT_MUTED},
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScatterInspectionUiState {
    pub(crate) hovered: Option<ScatterInspectionHit>,
    pub(crate) pinned: Option<PinnedScatterInspection>,
    pub(crate) x_label: String,
    pub(crate) y_label: String,
    pub(crate) hovered_difference: Option<DifferenceInspection>,
    pub(crate) pinned_difference: Option<DifferenceInspection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScatterInspectionAction {
    ClearPinned,
}

pub(crate) fn scatter_inspection_ui_state(app: &WorkbenchApp) -> Option<ScatterInspectionUiState> {
    let (x_label, y_label) = app
        .active_dataset_profile
        .and_then(|profile_id| dataset_profile(profile_id).scatter_binding)
        .map(|binding| (binding.x_column.to_string(), binding.y_column.to_string()))
        .unwrap_or_else(|| ("x".to_string(), "y".to_string()));
    (app.demo_mode.is_scatter()
        && app.visible_surface == WorkbenchSurface::Primary
        && app.scatter_filters.evaluation.is_some())
    .then(|| ScatterInspectionUiState {
        hovered: app.scatter_inspection.hovered.clone(),
        pinned: app.scatter_inspection.pinned.clone(),
        x_label,
        y_label,
        hovered_difference: app
            .scatter_inspection
            .hovered
            .as_ref()
            .and_then(|hit| difference_for_hit(app, hit)),
        pinned_difference: app
            .scatter_inspection
            .pinned
            .as_ref()
            .and_then(|pinned| difference_for_hit(app, &pinned.hit)),
    })
}

fn difference_for_hit(
    app: &WorkbenchApp,
    hit: &ScatterInspectionHit,
) -> Option<DifferenceInspection> {
    if app.scatter.density_mode != ScatterDensityMode::FilteredDifference {
        return None;
    }
    let baseline = app
        .scatter_inspection
        .baseline_grid
        .as_ref()?
        .inspect_bin(hit.bin_x, hit.bin_y)?;
    let active_total = app.scatter_filters.evaluation.as_ref()?.included_count as u64;
    Some(difference_inspection(
        baseline.count,
        hit.count,
        app.scatter.points.len() as u64,
        active_total,
    ))
}

pub(crate) fn show_scatter_inspection_tooltip(
    context: &Context,
    state: Option<&ScatterInspectionUiState>,
) {
    let Some(state) = state else { return };
    let Some(hit) = state.hovered.as_ref() else {
        return;
    };
    let Some(pointer) = context.pointer_hover_pos() else {
        return;
    };
    Area::new(Id::new("scatter_inspection_tooltip"))
        .order(Order::Tooltip)
        .fixed_pos(pointer + vec2(14.0, 14.0))
        .interactable(false)
        .show(context, |ui| {
            Frame::popup(ui.style())
                .show(ui, |ui| show_hit(ui, state, hit, state.hovered_difference));
        });
}

pub(crate) fn show_pinned_scatter_inspection(
    ui: &mut Ui,
    state: Option<&ScatterInspectionUiState>,
) -> Option<ScatterInspectionAction> {
    let state = state?;
    let pinned = state.pinned.as_ref()?;
    let mut action = None;
    ui.horizontal(|ui| {
        ui.heading("Pinned Inspection");
        if ui.small_button("Clear").clicked() {
            action = Some(ScatterInspectionAction::ClearPinned);
        }
    });
    show_hit(ui, state, &pinned.hit, state.pinned_difference);
    for summary in &pinned.category_summaries {
        let values = summary
            .value_counts
            .iter()
            .map(|(value, count)| format!("{value} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        ui.label(format!("{}: {values}", summary.column_name));
    }
    if !pinned.source_rows.is_empty() {
        ui.label(
            RichText::new(
                pinned
                    .source_rows
                    .iter()
                    .map(|row| row.row_id.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .monospace()
            .small()
            .color(TEXT_MUTED),
        );
    }
    action
}

fn show_hit(
    ui: &mut Ui,
    state: &ScatterInspectionUiState,
    hit: &ScatterInspectionHit,
    difference: Option<DifferenceInspection>,
) {
    ui.label(
        RichText::new(format!("{} rows", hit.count))
            .strong()
            .color(ACCENT),
    );
    if let Some(difference) = difference {
        ui.label(format!(
            "Active {} ({:.3}%) | baseline {} ({:.3}%)",
            difference.active_count,
            difference.active_share * 100.0,
            difference.baseline_count,
            difference.baseline_share * 100.0,
        ));
        ui.label(format!("Share delta {:+.4}%", difference.delta * 100.0));
    }
    ui.label(format!(
        "{} {:.3}..{:.3}",
        state.x_label, hit.x_range.min, hit.x_range.max
    ));
    ui.label(format!(
        "{} {:.3}..{:.3}",
        state.y_label, hit.y_range.min, hit.y_range.max
    ));
    ui.label(
        RichText::new(sample_disclosure(hit))
            .small()
            .color(TEXT_MUTED),
    );
}

fn sample_disclosure(hit: &ScatterInspectionHit) -> String {
    let sample_count = hit.row_ids.len();
    if sample_count < hit.count as usize {
        format!("{sample_count} sampled rows of {} in bin", hit.count)
    } else {
        format!("{sample_count} rows in bin")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rawscope_core::{F32Range, RowId};

    use super::*;

    #[test]
    fn pin_discloses_bounded_sample() {
        let hit = ScatterInspectionHit {
            bin_x: 0,
            bin_y: 0,
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
            count: 842,
            row_ids: Arc::from([RowId(1), RowId(2)]),
        };

        assert_eq!(sample_disclosure(&hit), "2 sampled rows of 842 in bin");
    }
}
