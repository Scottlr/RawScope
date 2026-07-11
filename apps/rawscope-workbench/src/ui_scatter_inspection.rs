//! Bounded tooltip and pinned content for scatter-density inspection.

use egui::{pos2, Rect};
use rawscope_data::dataset_profile;
use rawscope_render::{
    DifferenceInspectionSummary, ScatterDensityMode, ScatterInspectionHit, ScatterInspectionSummary,
};

use crate::{
    app::WorkbenchApp,
    app_scatter_inspection::{difference_summary_for_hit, PinnedScatterInspection},
    ui::WorkbenchSurface,
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
    scatter_inspection_ui_state_with_pin(app, true)
}

pub(crate) fn scatter_inspection_tooltip_state(
    app: &WorkbenchApp,
) -> Option<ScatterInspectionUiState> {
    scatter_inspection_ui_state_with_pin(app, false)
}

fn scatter_inspection_ui_state_with_pin(
    app: &WorkbenchApp,
    include_pinned: bool,
) -> Option<ScatterInspectionUiState> {
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
        pinned: include_pinned
            .then(|| app.scatter_inspection.pinned.clone())
            .flatten(),
        x_label,
        y_label,
        hovered_summary,
        hovered_difference: app
            .scatter_inspection
            .hovered
            .as_ref()
            .filter(|hit| inspection_hit_is_meaningful(app, hit))
            .and_then(|hit| difference_summary_for_hit(app, hit)),
        pinned_difference: app
            .scatter_inspection
            .pinned
            .as_ref()
            .and_then(|pinned| pinned.difference.clone()),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rawscope_core::{F32Range, RowId};
    use rawscope_data::{FilterMask, ScatterPointKind, ScatterPointRecord};
    use rawscope_render::{build_scatter_inspection_grid, ScatterInspectionConfig};

    use super::*;

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
