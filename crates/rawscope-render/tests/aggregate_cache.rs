use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord};
use rawscope_render::{
    scatter_aggregate_overview, timeline_aggregate_overview, AggregateCacheConfig,
    AggregateCacheError,
};

fn scatter_point(row_id: u64, x: f32, y: f32) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind: ScatterPointKind::Unclassified,
    }
}

fn timeline_event(row_id: u64, timestamp: u64, lane: u32) -> TimelineEventRecord {
    TimelineEventRecord {
        row_id: RowId(row_id),
        timestamp,
        lane,
        value: 0.0,
        kind: TimelineEventKind::Unclassified,
    }
}

#[test]
fn scatter_overview_counts_all_records() {
    let points = [
        scatter_point(1, 0.0, 0.0),
        scatter_point(2, 1.999, 2.0),
        scatter_point(3, 2.0, 4.0),
        scatter_point(4, 8.0, 8.0),
    ];

    let overview = scatter_aggregate_overview(
        &points,
        F32Range::new(0.0, 8.0),
        F32Range::new(0.0, 8.0),
        AggregateCacheConfig {
            grid_width: 4,
            grid_height: 4,
            max_row_ids_per_bin: 8,
        },
    )
    .expect("scatter overview should build");

    assert_eq!(overview.bins.len(), 16);
    assert_eq!(overview.max_bin_count, 1);
    assert_eq!(overview.bins[0].count, 1);
    assert_eq!(overview.bins[4].count, 1);
    assert_eq!(overview.bins[9].count, 1);
    assert_eq!(overview.bins[15].count, 1);
    assert_eq!(overview.bins[0].row_ids, vec![RowId(1)]);
    assert_eq!(overview.bins[4].row_ids, vec![RowId(2)]);
    assert_eq!(overview.bins[9].row_ids, vec![RowId(3)]);
    assert_eq!(overview.bins[15].row_ids, vec![RowId(4)]);
}

#[test]
fn scatter_overview_limits_row_samples() {
    let points = [
        scatter_point(7, 1.0, 1.0),
        scatter_point(2, 1.5, 1.5),
        scatter_point(9, 1.75, 1.75),
    ];

    let overview = scatter_aggregate_overview(
        &points,
        F32Range::new(0.0, 10.0),
        F32Range::new(0.0, 10.0),
        AggregateCacheConfig {
            grid_width: 2,
            grid_height: 2,
            max_row_ids_per_bin: 2,
        },
    )
    .expect("scatter overview should build");

    assert_eq!(overview.bins.len(), 4);
    assert_eq!(overview.bins[0].count, 3);
    assert_eq!(overview.bins[0].row_ids, vec![RowId(7), RowId(2)]);
    assert!(overview.bins[1].row_ids.is_empty());
    assert!(overview.bins[2].row_ids.is_empty());
    assert!(overview.bins[3].row_ids.is_empty());
}

#[test]
fn timeline_overview_counts_by_time_and_lane() {
    let events = [
        timeline_event(1, 100, 0),
        timeline_event(2, 199, 1),
        timeline_event(3, 250, 1),
        timeline_event(4, 499, 2),
        timeline_event(5, 500, 2),
    ];

    let overview = timeline_aggregate_overview(
        &events,
        U64Range::new(100, 500),
        3,
        AggregateCacheConfig {
            grid_width: 4,
            grid_height: 3,
            max_row_ids_per_bin: 8,
        },
    )
    .expect("timeline overview should build");

    assert_eq!(overview.bins.len(), 12);
    assert_eq!(overview.max_bin_count, 2);
    assert_eq!(overview.bins[0].count, 1);
    assert_eq!(overview.bins[4].count, 1);
    assert_eq!(overview.bins[5].count, 1);
    assert_eq!(overview.bins[11].count, 2);
    assert_eq!(overview.bins[11].row_ids, vec![RowId(4), RowId(5)]);
}

#[test]
fn timeline_overview_is_deterministic() {
    let events = [
        timeline_event(9, 100, 0),
        timeline_event(4, 100, 0),
        timeline_event(7, 100, 0),
        timeline_event(2, 100, 0),
    ];

    let config = AggregateCacheConfig {
        grid_width: 2,
        grid_height: 2,
        max_row_ids_per_bin: 2,
    };

    let first = timeline_aggregate_overview(&events, U64Range::new(100, 200), 1, config)
        .expect("timeline overview should build");
    let second = timeline_aggregate_overview(&events, U64Range::new(100, 200), 1, config)
        .expect("timeline overview should build");

    assert_eq!(first, second);
    assert_eq!(first.bins[0].row_ids, vec![RowId(9), RowId(4)]);
}

#[test]
fn zero_sized_or_invalid_config_is_rejected() {
    let points = [scatter_point(1, 1.0, 1.0)];
    let events = [timeline_event(1, 1, 0)];

    let scatter_err = scatter_aggregate_overview(
        &points,
        F32Range::new(0.0, 2.0),
        F32Range::new(0.0, 2.0),
        AggregateCacheConfig {
            grid_width: 0,
            grid_height: 4,
            max_row_ids_per_bin: 4,
        },
    )
    .expect_err("zero-width scatter grid should be rejected");
    assert!(matches!(scatter_err, AggregateCacheError::EmptyGrid));

    let scatter_err = scatter_aggregate_overview(
        &points,
        F32Range::new(0.0, 2.0),
        F32Range::new(0.0, 2.0),
        AggregateCacheConfig {
            grid_width: 4,
            grid_height: 0,
            max_row_ids_per_bin: 4,
        },
    )
    .expect_err("zero-height scatter grid should be rejected");
    assert!(matches!(scatter_err, AggregateCacheError::EmptyGrid));

    let timeline_err = timeline_aggregate_overview(
        &events,
        U64Range::new(0, 2),
        0,
        AggregateCacheConfig {
            grid_width: 4,
            grid_height: 4,
            max_row_ids_per_bin: 4,
        },
    )
    .expect_err("zero lane count should be rejected");
    assert!(matches!(
        timeline_err,
        AggregateCacheError::EmptyTimelineLanes
    ));

    let timeline_err = timeline_aggregate_overview(
        &events,
        U64Range::new(0, 2),
        1,
        AggregateCacheConfig {
            grid_width: 0,
            grid_height: 4,
            max_row_ids_per_bin: 4,
        },
    )
    .expect_err("zero-width timeline grid should be rejected");
    assert!(matches!(timeline_err, AggregateCacheError::EmptyGrid));
}
