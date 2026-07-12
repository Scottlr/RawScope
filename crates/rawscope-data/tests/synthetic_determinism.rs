use rawscope_core::{F32Range, U64Range};
use rawscope_data::{
    generate_synthetic_events, generate_synthetic_points, try_generate_synthetic_events,
    ScatterPointKind, SyntheticEventConfig, SyntheticEventConfigError, SyntheticEventType,
    SyntheticPointCategory, SyntheticPointConfig, TimelineEventKind,
};

#[test]
fn same_seed_produces_same_point_data() {
    let config = SyntheticPointConfig::new(42, 128);

    let first = generate_synthetic_points(config);
    let second = generate_synthetic_points(config);

    assert_eq!(first, second);
    assert_eq!(first.points.first().map(|point| point.row_id.0), Some(0));
    assert_eq!(first.points.last().map(|point| point.row_id.0), Some(127));
}

#[test]
fn point_generator_includes_dense_cluster_and_sparse_outliers() {
    let dataset = generate_synthetic_points(SyntheticPointConfig {
        seed: 7,
        row_count: 200,
        x_range: F32Range::new(0.0, 100.0),
        y_range: F32Range::new(0.0, 100.0),
    });

    let cluster_count = dataset
        .points
        .iter()
        .filter(|point| point.kind == ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster))
        .count();
    let outlier_count = dataset
        .points
        .iter()
        .filter(|point| point.kind == ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier))
        .count();

    assert!(cluster_count > outlier_count);
    assert!(outlier_count > 0);
    assert!(dataset
        .points
        .iter()
        .any(|point| point.x < 2.5 || point.x > 97.5));
}

#[test]
fn same_seed_produces_same_event_data() {
    let config = SyntheticEventConfig::new(99, 256);

    let first = generate_synthetic_events(config);
    let second = generate_synthetic_events(config);

    assert_eq!(first, second);
    assert_eq!(first.events.first().map(|event| event.row_id.0), Some(0));
    assert_eq!(first.events.last().map(|event| event.row_id.0), Some(255));
}

#[test]
fn event_generator_includes_gap_stale_lane_and_high_value_band() {
    let dataset = generate_synthetic_events(SyntheticEventConfig {
        seed: 11,
        row_count: 400,
        time_range: U64Range::new(0, 1_000),
        lane_count: 8,
    });

    assert!(dataset
        .events
        .iter()
        .all(|event| event.timestamp < 220 || event.timestamp > 280));

    assert!(dataset.events.iter().any(|event| {
        event.kind == TimelineEventKind::Synthetic(SyntheticEventType::StaleLane)
            && event.timestamp <= 120
    }));

    assert!(dataset.events.iter().any(|event| {
        event.kind == TimelineEventKind::Synthetic(SyntheticEventType::HighValueBand)
            && event.value >= 80.0
    }));
}

#[test]
fn small_event_requests_generate_exact_counts() {
    for row_count in 0..=3 {
        let dataset = generate_synthetic_events(SyntheticEventConfig::new(17, row_count));
        assert_eq!(dataset.events.len(), row_count);
        assert_eq!(
            dataset.events.last().map(|event| event.row_id.0),
            row_count.checked_sub(1).map(|value| value as u64)
        );
    }
}

#[test]
fn invalid_synthetic_event_windows_return_typed_errors() {
    let invalid_lane_count = try_generate_synthetic_events(SyntheticEventConfig {
        seed: 1,
        row_count: 1,
        time_range: U64Range::new(0, 1000),
        lane_count: 1,
    });
    assert!(matches!(
        invalid_lane_count,
        Err(SyntheticEventConfigError::LaneCountTooSmall { actual: 1 })
    ));

    let invalid_window = try_generate_synthetic_events(SyntheticEventConfig {
        seed: 1,
        row_count: 16,
        time_range: U64Range::new(0, 100),
        lane_count: 4,
    });
    assert!(matches!(
        invalid_window,
        Err(SyntheticEventConfigError::WindowOutsideRange { .. })
    ));
}
