use rawscope_core::{RowId, U64Range};
use rawscope_data::{SyntheticEventType, TimelineEventKind, TimelineEventRecord};
use rawscope_render::{
    BrushScreenPoint, BrushScreenRect, BrushScreenSize, PlotPointPx, PlotRectPx, TimelineBrushDrag,
    TimelineBrushSelection, TimelineLaneRange, TimelineSelectionSummary, TimelineViewport,
};

fn viewport() -> TimelineViewport {
    TimelineViewport::new(U64Range::new(0, 1_000), 8)
}

fn event(
    row_id: u64,
    timestamp: u64,
    lane: u32,
    value: f32,
    event_type: SyntheticEventType,
) -> TimelineEventRecord {
    TimelineEventRecord {
        row_id: RowId(row_id),
        timestamp,
        lane,
        value,
        kind: TimelineEventKind::Synthetic(event_type),
    }
}

fn unclassified_event(row_id: u64, timestamp: u64, lane: u32, value: f32) -> TimelineEventRecord {
    TimelineEventRecord {
        row_id: RowId(row_id),
        timestamp,
        lane,
        value,
        kind: TimelineEventKind::Unclassified,
    }
}

#[test]
fn screen_brush_converts_to_time_and_lane_selection() {
    let screen_rect = BrushScreenRect::from_points(
        BrushScreenPoint::new(250.0, 125.0),
        BrushScreenPoint::new(750.0, 375.0),
        BrushScreenSize::new(1_000.0, 800.0),
    )
    .unwrap();

    let selection = TimelineBrushSelection::from_screen_rect(
        screen_rect,
        BrushScreenSize::new(1_000.0, 800.0),
        viewport(),
    )
    .unwrap();

    assert_eq!(selection.time_range, U64Range::new(250, 750));
    assert_eq!(selection.lane_range, TimelineLaneRange::new(1, 4));
}

#[test]
fn drag_finalizes_screen_brush_to_data_space_selection() {
    let screen_rect = BrushScreenRect::from_points(
        BrushScreenPoint::new(100.0, 0.0),
        BrushScreenPoint::new(300.0, 100.0),
        BrushScreenSize::new(1_000.0, 800.0),
    )
    .unwrap();
    let drag = TimelineBrushDrag::from_screen_rect(screen_rect);

    let selection = drag
        .finalize(BrushScreenSize::new(1_000.0, 800.0), viewport())
        .unwrap();

    assert_eq!(selection.time_range, U64Range::new(100, 300));
    assert_eq!(selection.lane_range, TimelineLaneRange::new(0, 1));
}

#[test]
fn timeline_brush_round_trips_through_offset_plot_rect() {
    let plot_rect = PlotRectPx::try_new(120, 80, 1_000, 800, 1_300, 1_000).unwrap();
    let start = plot_rect
        .local_point(PlotPointPx::new(370.0, 205.0))
        .unwrap();
    let end = plot_rect
        .local_point(PlotPointPx::new(870.0, 455.0))
        .unwrap();
    let screen_rect = BrushScreenRect::from_points(start, end, plot_rect.screen_size()).unwrap();
    let selection =
        TimelineBrushSelection::from_screen_rect(screen_rect, plot_rect.screen_size(), viewport())
            .unwrap();

    let projected = selection
        .project_to_screen(viewport(), plot_rect.screen_size())
        .unwrap();

    assert_eq!(projected.min_x + plot_rect.x as f32, 370.0);
    assert_eq!(projected.min_y + plot_rect.y as f32, 180.0);
    assert_eq!(projected.max_x + plot_rect.x as f32, 870.0);
    assert_eq!(projected.max_y + plot_rect.y as f32, 480.0);
}

#[test]
fn lane_mapping_uses_top_to_bottom_half_open_ranges() {
    let screen_size = BrushScreenSize::new(1_000.0, 800.0);
    let top_rect = BrushScreenRect::from_points(
        BrushScreenPoint::new(0.0, 0.0),
        BrushScreenPoint::new(100.0, 100.0),
        screen_size,
    )
    .unwrap();
    let bottom_rect = BrushScreenRect::from_points(
        BrushScreenPoint::new(0.0, 700.0),
        BrushScreenPoint::new(100.0, 800.0),
        screen_size,
    )
    .unwrap();

    let top_selection =
        TimelineBrushSelection::from_screen_rect(top_rect, screen_size, viewport()).unwrap();
    let bottom_selection =
        TimelineBrushSelection::from_screen_rect(bottom_rect, screen_size, viewport()).unwrap();

    assert_eq!(top_selection.lane_range, TimelineLaneRange::new(0, 1));
    assert_eq!(bottom_selection.lane_range, TimelineLaneRange::new(7, 8));
}

#[test]
fn offscreen_projection_hides_selection() {
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(0, 100),
        lane_range: TimelineLaneRange::new(0, 1),
    };
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.8, 0.2);

    assert_eq!(
        selection.project_to_screen(viewport, BrushScreenSize::new(1_000.0, 800.0)),
        None
    );
}

#[test]
fn partial_projection_clamps_to_viewport_edge() {
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(100, 700),
        lane_range: TimelineLaneRange::new(2, 6),
    };
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.5, 0.5);

    let rect = selection
        .project_to_screen(viewport, BrushScreenSize::new(1_000.0, 800.0))
        .unwrap();

    assert_eq!(rect.min_x, 0.0);
    assert_eq!(rect.max_x, 900.0);
    assert_eq!(rect.min_y, 200.0);
    assert_eq!(rect.max_y, 600.0);
}

#[test]
fn zoom_and_pan_reproject_data_anchored_selection() {
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(400, 600),
        lane_range: TimelineLaneRange::new(1, 3),
    };
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.5, 0.5);
    let zoomed_rect = selection
        .project_to_screen(viewport, BrushScreenSize::new(1_000.0, 800.0))
        .unwrap();

    viewport.pan_by(100);
    let panned_rect = selection
        .project_to_screen(viewport, BrushScreenSize::new(1_000.0, 800.0))
        .unwrap();

    assert_eq!(zoomed_rect.min_x, 300.0);
    assert_eq!(zoomed_rect.max_x, 700.0);
    assert_eq!(panned_rect.min_x, 100.0);
    assert_eq!(panned_rect.max_x, 500.0);
}

#[test]
fn selected_summary_counts_events_lanes_and_types() {
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(100, 300),
        lane_range: TimelineLaneRange::new(1, 3),
    };
    let events = vec![
        event(0, 120, 1, 10.0, SyntheticEventType::Background),
        event(1, 180, 1, 20.0, SyntheticEventType::Spike),
        event(2, 250, 2, 30.0, SyntheticEventType::Spike),
        event(3, 260, 4, 40.0, SyntheticEventType::HighValueBand),
        event(4, 400, 1, 50.0, SyntheticEventType::StaleLane),
    ];

    let summary = TimelineSelectionSummary::from_events(&events, selection, 5);

    assert_eq!(summary.selected_event_count, 3);
    assert_eq!(summary.total_event_count, 5);
    assert!((summary.selected_percentage - 60.0).abs() < 0.001);
    assert_eq!(summary.lane_counts, vec![0, 2, 1, 0, 0]);
    assert_eq!(summary.event_type_counts.background, 1);
    assert_eq!(summary.event_type_counts.spike, 2);
    assert_eq!(summary.event_type_counts.unclassified, 0);
    assert_eq!(summary.top_lane, Some(1));
    assert_eq!(summary.top_event_type, Some(SyntheticEventType::Spike));
    assert_eq!(
        summary.selected_timestamp_range,
        Some(U64Range::new(120, 250))
    );
    assert_eq!(summary.selected_value_range, Some((10.0, 30.0)));
}

#[test]
fn empty_selection_summary_has_zero_counts() {
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(900, 950),
        lane_range: TimelineLaneRange::new(0, 1),
    };
    let events = vec![event(0, 120, 1, 10.0, SyntheticEventType::Background)];

    let summary = TimelineSelectionSummary::from_events(&events, selection, 4);

    assert_eq!(summary.selected_event_count, 0);
    assert_eq!(summary.lane_counts, vec![0, 0, 0, 0]);
    assert_eq!(summary.top_lane, None);
    assert_eq!(summary.top_event_type, None);
    assert_eq!(summary.selected_timestamp_range, None);
    assert_eq!(summary.selected_value_range, None);
}

#[test]
fn selected_summary_tracks_unclassified_local_events_without_synthetic_top_type() {
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(100, 300),
        lane_range: TimelineLaneRange::new(0, 2),
    };
    let events = vec![
        unclassified_event(0, 120, 0, 1.0),
        unclassified_event(1, 150, 1, 2.0),
    ];

    let summary = TimelineSelectionSummary::from_events(&events, selection, 2);

    assert_eq!(summary.event_type_counts.background, 0);
    assert_eq!(summary.event_type_counts.spike, 0);
    assert_eq!(summary.event_type_counts.stale_lane, 0);
    assert_eq!(summary.event_type_counts.high_value_band, 0);
    assert_eq!(summary.event_type_counts.unclassified, 2);
    assert_eq!(summary.top_event_type, None);
}
