use rawscope_core::{F32Range, RowId};
use rawscope_data::{ScatterPointKind, ScatterPointRecord, SyntheticPointCategory};
use rawscope_render::{
    BrushScreenPoint, BrushScreenRect, BrushScreenSize, ScatterBrushDrag, ScatterBrushSelection,
    ScatterViewport, SelectedRegionSummary,
};

fn viewport() -> ScatterViewport {
    ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 100.0))
}

fn point(row_id: u64, x: f32, y: f32, category: SyntheticPointCategory) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind: ScatterPointKind::Synthetic(category),
    }
}

fn unclassified_point(row_id: u64, x: f32, y: f32) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind: ScatterPointKind::Unclassified,
    }
}

#[test]
fn screen_rectangle_normalizes_drag_direction() {
    let rect = BrushScreenRect::from_points(
        BrushScreenPoint::new(80.0, 70.0),
        BrushScreenPoint::new(10.0, 20.0),
        BrushScreenSize::new(100.0, 100.0),
    )
    .unwrap();

    assert_eq!(rect.min_x, 10.0);
    assert_eq!(rect.min_y, 20.0);
    assert_eq!(rect.max_x, 80.0);
    assert_eq!(rect.max_y, 70.0);
}

#[test]
fn screen_rectangle_clamps_outside_viewport() {
    let rect = BrushScreenRect::from_points(
        BrushScreenPoint::new(-10.0, -20.0),
        BrushScreenPoint::new(120.0, 130.0),
        BrushScreenSize::new(100.0, 100.0),
    )
    .unwrap();

    assert_eq!(rect.min_x, 0.0);
    assert_eq!(rect.min_y, 0.0);
    assert_eq!(rect.max_x, 100.0);
    assert_eq!(rect.max_y, 100.0);
}

#[test]
fn brush_converts_screen_rect_to_data_ranges() {
    let brush = ScatterBrushSelection::from_screen_points(
        BrushScreenPoint::new(25.0, 25.0),
        BrushScreenPoint::new(75.0, 75.0),
        BrushScreenSize::new(100.0, 100.0),
        viewport(),
    )
    .unwrap();

    assert_eq!(brush.x_range, F32Range::new(25.0, 75.0));
    assert_eq!(brush.y_range, F32Range::new(25.0, 75.0));
}

#[test]
fn drag_finalizes_screen_brush_to_data_space_range() {
    let drag = ScatterBrushDrag::from_screen_points(
        BrushScreenPoint::new(10.0, 20.0),
        BrushScreenPoint::new(40.0, 70.0),
        BrushScreenSize::new(100.0, 100.0),
    )
    .unwrap();

    let selection = drag
        .finalize(BrushScreenSize::new(100.0, 100.0), viewport())
        .unwrap();

    assert_eq!(selection.x_range, F32Range::new(10.0, 40.0));
    assert_eq!(selection.y_range, F32Range::new(30.0, 80.0));
}

#[test]
fn data_space_brush_projects_after_zoom() {
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(25.0, 75.0),
        y_range: F32Range::new(25.0, 75.0),
    };
    let mut viewport = viewport();
    viewport.zoom_around(50.0, 50.0, 0.5);

    let rect = selection
        .project_to_screen(viewport, BrushScreenSize::new(100.0, 100.0))
        .unwrap();

    assert_eq!(rect.min_x, 0.0);
    assert_eq!(rect.max_x, 100.0);
    assert_eq!(rect.min_y, 0.0);
    assert_eq!(rect.max_y, 100.0);
}

#[test]
fn data_space_brush_projects_after_pan() {
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(40.0, 60.0),
        y_range: F32Range::new(40.0, 60.0),
    };
    let mut viewport = viewport();
    viewport.zoom_around(50.0, 50.0, 0.5);
    viewport.pan_by(25.0, 0.0);

    let rect = selection
        .project_to_screen(viewport, BrushScreenSize::new(100.0, 100.0))
        .unwrap();

    assert_eq!(rect.min_x, 0.0);
    assert_eq!(rect.max_x, 20.0);
    assert_eq!(rect.min_y, 30.000002);
    assert_eq!(rect.max_y, 70.0);
}

#[test]
fn reset_viewport_preserves_data_space_selection_projection() {
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(25.0, 75.0),
        y_range: F32Range::new(25.0, 75.0),
    };
    let mut viewport = viewport();
    viewport.zoom_around(50.0, 50.0, 0.5);
    viewport.reset();

    let rect = selection
        .project_to_screen(viewport, BrushScreenSize::new(100.0, 100.0))
        .unwrap();

    assert_eq!(rect.min_x, 25.0);
    assert_eq!(rect.max_x, 75.0);
    assert_eq!(rect.min_y, 25.0);
    assert_eq!(rect.max_y, 75.0);
}

#[test]
fn offscreen_data_space_brush_projection_is_hidden() {
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(0.0, 10.0),
        y_range: F32Range::new(0.0, 10.0),
    };
    let mut viewport = viewport();
    viewport.zoom_around(75.0, 75.0, 0.5);

    assert_eq!(
        selection.project_to_screen(viewport, BrushScreenSize::new(100.0, 100.0)),
        None
    );
}

#[test]
fn brush_clear_reset_is_represented_by_absent_selection() {
    let mut brush = Some(
        ScatterBrushSelection::from_screen_points(
            BrushScreenPoint::new(10.0, 10.0),
            BrushScreenPoint::new(20.0, 20.0),
            BrushScreenSize::new(100.0, 100.0),
            viewport(),
        )
        .unwrap(),
    );
    assert!(brush.is_some());

    brush = None;

    assert_eq!(brush, None);
}

#[test]
fn selected_summary_counts_rows_in_brush() {
    let brush = ScatterBrushSelection::from_screen_points(
        BrushScreenPoint::new(0.0, 50.0),
        BrushScreenPoint::new(50.0, 100.0),
        BrushScreenSize::new(100.0, 100.0),
        viewport(),
    )
    .unwrap();
    let points = vec![
        point(0, 10.0, 10.0, SyntheticPointCategory::Cluster),
        point(1, 40.0, 40.0, SyntheticPointCategory::Background),
        point(2, 90.0, 90.0, SyntheticPointCategory::Outlier),
    ];

    let summary = SelectedRegionSummary::from_points(&points, brush);

    assert_eq!(summary.selected_row_count, 2);
    assert_eq!(summary.total_row_count, 3);
    assert!((summary.selected_percentage - 66.66667).abs() < 0.001);
}

#[test]
fn selected_summary_counts_categories() {
    let brush = ScatterBrushSelection::from_screen_points(
        BrushScreenPoint::new(0.0, 0.0),
        BrushScreenPoint::new(100.0, 100.0),
        BrushScreenSize::new(100.0, 100.0),
        viewport(),
    )
    .unwrap();
    let points = vec![
        point(0, 10.0, 10.0, SyntheticPointCategory::Cluster),
        point(1, 20.0, 20.0, SyntheticPointCategory::Cluster),
        point(2, 30.0, 30.0, SyntheticPointCategory::Outlier),
    ];

    let summary = SelectedRegionSummary::from_points(&points, brush);

    assert_eq!(summary.category_counts.cluster, 2);
    assert_eq!(summary.category_counts.background, 0);
    assert_eq!(summary.category_counts.outlier, 1);
    assert_eq!(summary.category_counts.unclassified, 0);
    assert_eq!(summary.top_category, Some(SyntheticPointCategory::Cluster));
}

#[test]
fn selected_summary_tracks_unclassified_local_points_without_synthetic_top_category() {
    let brush = ScatterBrushSelection::from_screen_points(
        BrushScreenPoint::new(0.0, 0.0),
        BrushScreenPoint::new(100.0, 100.0),
        BrushScreenSize::new(100.0, 100.0),
        viewport(),
    )
    .unwrap();
    let points = vec![
        unclassified_point(0, 10.0, 10.0),
        unclassified_point(1, 20.0, 20.0),
    ];

    let summary = SelectedRegionSummary::from_points(&points, brush);

    assert_eq!(summary.category_counts.cluster, 0);
    assert_eq!(summary.category_counts.background, 0);
    assert_eq!(summary.category_counts.outlier, 0);
    assert_eq!(summary.category_counts.unclassified, 2);
    assert_eq!(summary.top_category, None);
}
