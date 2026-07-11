use std::path::PathBuf;

use egui::{pos2, Rect};
use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    build_visual_field_catalog, evaluate_filters, DatasetEvidenceKey, FilterSet, LoadedColumnKind,
    LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind, ScatterPointRecord,
    VisualFieldCatalogConfig,
};
use rawscope_render::{
    build_scatter_inspection_grid, PlotRectPx, ScatterInspectionConfig, ScatterViewport,
};
use winit::dpi::PhysicalPosition;

use crate::app_session::ActiveSessionContext;

use super::*;

#[test]
fn stale_revision_disables_hover_hit() {
    let mut app = inspection_app();
    app.refresh_scatter_inspection_hover();
    assert!(app.scatter_inspection.hovered.is_some());
    app.scatter_inspection.cache_filter_revision = FilterRevision(99);

    app.refresh_scatter_inspection_hover();

    assert!(app.scatter_inspection.hovered.is_none());
    assert!(app.scatter_inspection.hovered_summary.is_none());
}

#[test]
fn settled_hover_consumes_cached_inspection_summary() {
    let mut app = inspection_app();
    app.refresh_scatter_inspection_hover();

    let summary = app
        .scatter_inspection
        .hovered_summary
        .as_ref()
        .expect("current hover should project its settled summary");
    assert_eq!(summary.hit.count, 1);
    assert_eq!(summary.neighborhood_count, 1);
}

#[test]
fn filter_revision_clears_stale_pin() {
    let mut app = inspection_app();
    app.refresh_scatter_inspection_hover();
    app.scatter_inspection.pinned = Some(test_pin(&app));

    app.invalidate_scatter_inspection();

    assert!(app.scatter_inspection.pinned.is_none());
}

#[test]
fn presentation_change_preserves_valid_pin() {
    let mut app = inspection_app();
    app.refresh_scatter_inspection_hover();
    app.scatter_inspection.pinned = Some(test_pin(&app));

    app.set_point_reveal_mode(rawscope_render::PointRevealMode::Off);

    assert!(app.scatter_inspection.pinned.is_some());
}

#[test]
fn pinned_evidence_keys_use_only_bounded_source_rows() {
    let mut app = WorkbenchApp::default();
    app.active_session = Some(ActiveSessionContext {
        manifest_path: PathBuf::from("analysis.rawscope.json"),
        display_name: None,
        data_format: rawscope_session::SessionDataFormat::Csv,
        evidence_key: Some(DatasetEvidenceKey {
            column_name: "game_id".into(),
            column_index: 1,
        }),
    });
    let rows = vec![
        LoadedSourceRow {
            row_id: RowId(4),
            values: vec!["white".into(), "g-4".into()],
        },
        LoadedSourceRow {
            row_id: RowId(9),
            values: vec!["black".into(), "g-9".into()],
        },
    ];

    let keys = app.pinned_evidence_key_values(&rows);

    assert_eq!(
        keys,
        vec![
            PinnedEvidenceKeyValue {
                column_name: "game_id".into(),
                value: "g-4".into(),
            },
            PinnedEvidenceKeyValue {
                column_name: "game_id".into(),
                value: "g-9".into(),
            },
        ]
    );
}

fn test_pin(app: &WorkbenchApp) -> PinnedScatterInspection {
    let summary = app
        .scatter_inspection
        .hovered_summary
        .clone()
        .expect("inspection fixture should have a settled summary");
    PinnedScatterInspection {
        summary,
        difference: None,
        category_summaries: Vec::new(),
        source_rows: Vec::new(),
        evidence_keys: Vec::new(),
        cache_viewport_revision: app.scatter_inspection.cache_viewport_revision,
        cache_filter_revision: app.scatter_inspection.cache_filter_revision,
    }
}

fn inspection_app() -> WorkbenchApp {
    let source = LoadedSourceTable {
        columns: vec![LoadedColumnSchema {
            name: "winner".into(),
            kind: LoadedColumnKind::String,
        }],
        rows: vec![LoadedSourceRow {
            row_id: RowId(0),
            values: vec!["white".into()],
        }],
    };
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    let evaluation = evaluate_filters(&source, &catalog, &FilterSet::default()).unwrap();
    let points = vec![ScatterPointRecord {
        row_id: RowId(0),
        x: 5.0,
        y: 5.0,
        kind: ScatterPointKind::Unclassified,
    }];
    let grid = build_scatter_inspection_grid(
        &points,
        &evaluation.mask,
        F32Range::new(0.0, 10.0),
        F32Range::new(0.0, 10.0),
        evaluation.revision,
        ScatterInspectionConfig {
            grid_width: 10,
            grid_height: 10,
            max_row_ids_per_bin: 16,
        },
    )
    .unwrap();
    WorkbenchApp {
        demo_mode: DemoMode::Scatter,
        interaction_mode: WorkbenchInteractionMode::Inspect,
        cursor_position: Some(PhysicalPosition::new(50.0, 50.0)),
        plot_surface: Some(crate::ui_plot_surface::PlotSurfaceLayout {
            logical_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)),
            physical_rect: PlotRectPx::try_new(0, 0, 100, 100, 100, 100).unwrap(),
            axis_layout: crate::ui_plot_surface::PlotAxisLayout {
                outer_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)),
                plot_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)),
            },
        }),
        scatter: crate::app::ScatterWorkbenchState {
            points,
            source_rows: Some(source),
            viewport: Some(ScatterViewport::new(
                F32Range::new(0.0, 10.0),
                F32Range::new(0.0, 10.0),
            )),
            ..Default::default()
        },
        scatter_filters: crate::app_scatter_filter::ScatterFilterState {
            catalog: Some(catalog),
            evaluation: Some(evaluation),
            ..Default::default()
        },
        scatter_inspection: ScatterInspectionState {
            grid: Some(grid),
            ..Default::default()
        },
        ..Default::default()
    }
}
