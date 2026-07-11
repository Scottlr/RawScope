//! Bounded tooltip and pinned content for scatter-density inspection.

use egui::{pos2, Rect, RichText, Ui};
use rawscope_data::dataset_profile;
use rawscope_render::{
    DifferenceDirection, DifferenceInspectionSummary, ScatterDensityMode, ScatterInspectionHit,
    ScatterInspectionSummary,
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
    pub(crate) hovered_summary: Option<ScatterInspectionSummary>,
    pub(crate) hovered_bin_rect: Option<Rect>,
    pub(crate) hovered_difference: Option<DifferenceInspectionSummary>,
    pub(crate) pinned_difference: Option<DifferenceInspectionSummary>,
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
    let hovered = app
        .scatter_inspection
        .hovered
        .clone()
        .filter(|hit| inspection_hit_is_meaningful(app, hit));
    let hovered_summary = hovered.as_ref().and_then(|hit| {
        app.scatter_inspection
            .hovered_summary
            .as_ref()
            .filter(|summary| summary.hit.bin_x == hit.bin_x && summary.hit.bin_y == hit.bin_y)
            .cloned()
    });
    (app.demo_mode.is_scatter()
        && app.visible_surface == WorkbenchSurface::Primary
        && app.scatter_filters.evaluation.is_some())
    .then(|| ScatterInspectionUiState {
        hovered_bin_rect: hovered.as_ref().and_then(|hit| logical_bin_rect(app, hit)),
        hovered,
        pinned: app.scatter_inspection.pinned.clone(),
        x_label,
        y_label,
        hovered_summary,
        hovered_difference: app
            .scatter_inspection
            .hovered
            .as_ref()
            .filter(|hit| inspection_hit_is_meaningful(app, hit))
            .and_then(|hit| difference_for_hit(app, hit)),
        pinned_difference: app
            .scatter_inspection
            .pinned
            .as_ref()
            .and_then(|pinned| difference_for_hit(app, &pinned.hit)),
    })
}

pub(crate) fn inspection_hit_is_meaningful(app: &WorkbenchApp, hit: &ScatterInspectionHit) -> bool {
    if app.scatter.density_mode == ScatterDensityMode::AbsoluteDensity {
        return hit.count > 0;
    }
    let baseline_count = app
        .scatter_inspection
        .baseline_grid
        .as_ref()
        .and_then(|grid| grid.inspect_bin(hit.bin_x, hit.bin_y))
        .map_or(0, |baseline| baseline.count);
    hit.count > 0 || baseline_count > 0
}

fn difference_for_hit(
    app: &WorkbenchApp,
    hit: &ScatterInspectionHit,
) -> Option<DifferenceInspectionSummary> {
    if app.scatter.density_mode != ScatterDensityMode::FilteredDifference {
        return None;
    }
    let baseline_count = app
        .scatter_inspection
        .baseline_grid
        .as_ref()?
        .inspect_bin(hit.bin_x, hit.bin_y)?
        .count;
    app.scatter_inspection
        .difference_distribution
        .as_ref()
        .map(|distribution| distribution.summarize_counts(baseline_count, hit.count))
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
    difference: Option<DifferenceInspectionSummary>,
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
        let direction = match difference.direction {
            DifferenceDirection::MoreCommonInActive => "More common in active",
            DifferenceDirection::LessCommonInActive => "Less common in active",
            DifferenceDirection::Unchanged => "Unchanged",
        };
        ui.label(format!(
            "{direction}: {:+.4} percentage points",
            difference.share_delta * 100.0
        ));
        if let Some(percentile) = difference.absolute_delta_percentile {
            ui.label(format!(
                "Difference strength: {:.1}th percentile",
                percentile * 100.0
            ));
        }
    }
    ui.label(format!(
        "{} {:.3}..{:.3}",
        state.x_label, hit.x_range.min, hit.x_range.max
    ));
    ui.label(format!(
        "{} {:.3}..{:.3}",
        state.y_label, hit.y_range.min, hit.y_range.max
    ));
    if !hit.row_ids.is_empty() {
        ui.label(
            RichText::new(sample_disclosure(hit))
                .small()
                .color(TEXT_MUTED),
        );
    }
}

fn logical_bin_rect(app: &WorkbenchApp, hit: &ScatterInspectionHit) -> Option<Rect> {
    let surface = app.plot_surface?;
    let grid = app.scatter_inspection.grid.as_ref()?;
    let local_rect = hit.screen_rect(
        grid.grid_width,
        grid.grid_height,
        surface.physical_rect.screen_size(),
    )?;
    let logical_plot = surface.axis_layout.plot_rect;
    let physical_width = surface.physical_rect.width as f32;
    let physical_height = surface.physical_rect.height as f32;
    if physical_width <= 0.0 || physical_height <= 0.0 {
        return None;
    }
    let x_scale = logical_plot.width() / physical_width;
    let y_scale = logical_plot.height() / physical_height;
    Some(Rect::from_min_max(
        pos2(
            logical_plot.left() + local_rect.min_x * x_scale,
            logical_plot.top() + local_rect.min_y * y_scale,
        ),
        pos2(
            logical_plot.left() + local_rect.max_x * x_scale,
            logical_plot.top() + local_rect.max_y * y_scale,
        ),
    ))
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
    use rawscope_data::{FilterMask, ScatterPointKind, ScatterPointRecord};
    use rawscope_render::{build_scatter_inspection_grid, ScatterInspectionConfig};

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

    #[test]
    fn absolute_empty_cell_is_suppressed_but_baseline_only_difference_is_visible() {
        let mut app = WorkbenchApp::default();
        let hit = ScatterInspectionHit {
            bin_x: 0,
            bin_y: 0,
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
            count: 0,
            row_ids: Arc::from([]),
        };
        assert!(!inspection_hit_is_meaningful(&app, &hit));

        let points = vec![ScatterPointRecord {
            row_id: RowId(0),
            x: 0.5,
            y: 0.5,
            kind: ScatterPointKind::Unclassified,
        }];
        let grid = build_scatter_inspection_grid(
            &points,
            &FilterMask::all_included(1),
            F32Range::new(0.0, 1.0),
            F32Range::new(0.0, 1.0),
            Default::default(),
            ScatterInspectionConfig {
                grid_width: 1,
                grid_height: 1,
                max_row_ids_per_bin: 4,
            },
        )
        .unwrap();
        app.scatter.density_mode = ScatterDensityMode::FilteredDifference;
        app.scatter_inspection.baseline_grid = Some(grid);

        assert!(inspection_hit_is_meaningful(&app, &hit));
    }
}
