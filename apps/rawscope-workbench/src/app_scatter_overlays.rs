//! Workbench projection of settled scatter inspection focus inputs.

use rawscope_render::{
    project_scatter_inspection_overlay, BrushScreenSize, InspectionFocusKind,
    ScatterInspectionOverlay,
};

use crate::app_scatter_inspection::{PinnedScatterInspection, ScatterInspectionState};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ScatterInspectionOverlayInputs {
    pub(crate) pinned: Option<ScatterInspectionOverlay>,
    pub(crate) hovered: Option<ScatterInspectionOverlay>,
}

pub(crate) fn scatter_inspection_overlay_inputs(
    state: &ScatterInspectionState,
    screen_size: BrushScreenSize,
) -> ScatterInspectionOverlayInputs {
    let pinned = state
        .pinned
        .as_ref()
        .and_then(|pinned| project_pinned(pinned, state, screen_size));
    let hovered = state.hovered.as_ref().and_then(|hit| {
        let grid = state.grid.as_ref()?;
        project_scatter_inspection_overlay(
            hit,
            grid.grid_width,
            grid.grid_height,
            screen_size,
            InspectionFocusKind::Hover,
            1.0,
        )
    });
    ScatterInspectionOverlayInputs { pinned, hovered }
}

fn project_pinned(
    pinned: &PinnedScatterInspection,
    state: &ScatterInspectionState,
    screen_size: BrushScreenSize,
) -> Option<ScatterInspectionOverlay> {
    let grid = state.grid.as_ref()?;
    project_scatter_inspection_overlay(
        &pinned.hit,
        grid.grid_width,
        grid.grid_height,
        screen_size,
        InspectionFocusKind::Pinned,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rawscope_core::{F32Range, RowId};
    use rawscope_data::{
        build_visual_field_catalog, evaluate_filters, FilterSet, LoadedColumnKind,
        LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
        ScatterPointRecord, VisualFieldCatalogConfig,
    };
    use rawscope_render::{
        build_scatter_inspection_grid, ScatterInspectionConfig, ScatterInspectionHit,
    };

    use super::*;

    #[test]
    fn app_scatter_overlays_projects_hover_and_pin_as_distinct_focus_kinds() {
        let points = vec![ScatterPointRecord {
            row_id: RowId(0),
            x: 1.0,
            y: 1.0,
            kind: ScatterPointKind::Unclassified,
        }];
        let source = LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "value".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: vec![LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["one".into()],
            }],
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let evaluation = evaluate_filters(&source, &catalog, &FilterSet::default()).unwrap();
        let grid = build_scatter_inspection_grid(
            &points,
            &evaluation.mask,
            F32Range::new(0.0, 2.0),
            F32Range::new(0.0, 2.0),
            evaluation.revision,
            ScatterInspectionConfig {
                grid_width: 2,
                grid_height: 2,
                max_row_ids_per_bin: 4,
            },
        )
        .unwrap();
        let hit = grid.inspect_bin(0, 0).unwrap();
        let state = ScatterInspectionState {
            grid: Some(grid),
            hovered: Some(hit.clone()),
            pinned: Some(PinnedScatterInspection {
                hit,
                category_summaries: Vec::new(),
                source_rows: Vec::new(),
            }),
            ..Default::default()
        };

        let inputs = scatter_inspection_overlay_inputs(&state, BrushScreenSize::new(100.0, 100.0));

        assert_eq!(
            inputs.hovered.unwrap().focus_kind,
            InspectionFocusKind::Hover
        );
        assert_eq!(
            inputs.pinned.unwrap().focus_kind,
            InspectionFocusKind::Pinned
        );
    }

    #[test]
    fn app_scatter_overlays_keep_exact_rect_inside_plot_pixels() {
        let hit = ScatterInspectionHit {
            bin_x: 0,
            bin_y: 1,
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
            count: 1,
            row_ids: Arc::from([RowId(0)]),
        };
        let source = LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "value".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: vec![LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["one".into()],
            }],
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let evaluation = evaluate_filters(&source, &catalog, &FilterSet::default()).unwrap();
        let points = vec![ScatterPointRecord {
            row_id: RowId(0),
            x: 1.0,
            y: 1.0,
            kind: ScatterPointKind::Unclassified,
        }];
        let grid = build_scatter_inspection_grid(
            &points,
            &evaluation.mask,
            F32Range::new(0.0, 2.0),
            F32Range::new(0.0, 2.0),
            evaluation.revision,
            ScatterInspectionConfig {
                grid_width: 2,
                grid_height: 2,
                max_row_ids_per_bin: 4,
            },
        )
        .unwrap();
        let state = ScatterInspectionState {
            grid: Some(grid),
            hovered: Some(hit),
            ..Default::default()
        };

        let overlay = scatter_inspection_overlay_inputs(&state, BrushScreenSize::new(80.0, 40.0))
            .hovered
            .unwrap();

        assert_eq!(overlay.exact_rect.min_x, 0.0);
        assert_eq!(overlay.exact_rect.max_y, 20.0);
        assert!(overlay.neighborhood_rect.max_x <= 80.0);
        assert!(overlay.neighborhood_rect.max_y <= 40.0);
    }
}
