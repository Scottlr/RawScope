use rawscope_core::{F32Range, U64Range};
use rawscope_data::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticPointConfig, SyntheticPointRecord,
};
use rawscope_render::{scatter_density, timeline_density};

fn sum_timeline_bins(
    grid: &rawscope_core::DensityGrid,
    x_bins: std::ops::RangeInclusive<u32>,
    lane_count: u32,
) -> u32 {
    x_bins
        .flat_map(|x_bin| (0..lane_count).map(move |lane_bin| grid.bin(x_bin, lane_bin).row_count))
        .sum()
}

#[test]
fn scatter_density_counts_are_deterministic() {
    let dataset = generate_synthetic_points(SyntheticPointConfig::new(5, 256));

    let first = scatter_density(&dataset.points, dataset.x_range, dataset.y_range, 16, 16);
    let second = scatter_density(&dataset.points, dataset.x_range, dataset.y_range, 16, 16);

    assert_eq!(first, second);
}

#[test]
fn timeline_density_counts_are_deterministic() {
    let dataset = generate_synthetic_events(SyntheticEventConfig::new(9, 320));

    let first = timeline_density(
        &dataset.events,
        dataset.time_range,
        dataset.lane_count,
        20,
        dataset.lane_count,
    );
    let second = timeline_density(
        &dataset.events,
        dataset.time_range,
        dataset.lane_count,
        20,
        dataset.lane_count,
    );

    assert_eq!(first, second);
}

#[test]
fn total_scatter_binned_count_matches_input_when_all_points_are_in_range() {
    let dataset = generate_synthetic_points(SyntheticPointConfig::new(17, 180));

    let grid = scatter_density(&dataset.points, dataset.x_range, dataset.y_range, 12, 12);

    assert_eq!(grid.total_row_count(), dataset.points.len() as u64);
}

#[test]
fn total_timeline_binned_count_matches_input_when_all_events_are_in_range() {
    let dataset = generate_synthetic_events(SyntheticEventConfig::new(21, 240));

    let grid = timeline_density(
        &dataset.events,
        dataset.time_range,
        dataset.lane_count,
        25,
        dataset.lane_count,
    );

    assert_eq!(grid.total_row_count(), dataset.events.len() as u64);
}

#[test]
fn rows_outside_scatter_range_are_excluded() {
    let points = vec![
        SyntheticPointRecord {
            row_id: rawscope_core::RowId(0),
            x: 10.0,
            y: 10.0,
            category: rawscope_data::SyntheticPointCategory::Background,
        },
        SyntheticPointRecord {
            row_id: rawscope_core::RowId(1),
            x: 250.0,
            y: 10.0,
            category: rawscope_data::SyntheticPointCategory::Outlier,
        },
    ];

    let grid = scatter_density(
        &points,
        F32Range::new(0.0, 100.0),
        F32Range::new(0.0, 100.0),
        10,
        10,
    );

    assert_eq!(grid.total_row_count(), 1);
    assert_eq!(grid.bin(1, 1).row_count, 1);
}

#[test]
fn rows_outside_timeline_range_are_excluded() {
    let dataset = generate_synthetic_events(SyntheticEventConfig::new(4, 90));
    let narrow_range = U64Range::new(300, 700);

    let grid = timeline_density(&dataset.events, narrow_range, dataset.lane_count, 10, 8);
    let expected = dataset
        .events
        .iter()
        .filter(|event| narrow_range.contains(event.timestamp))
        .count() as u64;

    assert_eq!(grid.total_row_count(), expected);
}

#[test]
fn injected_point_cluster_is_visible_in_expected_scatter_bin() {
    let dataset = generate_synthetic_points(SyntheticPointConfig::new(12, 300));
    let grid = scatter_density(&dataset.points, dataset.x_range, dataset.y_range, 10, 10);

    let cluster_bin_count = grid.bin(2, 7).row_count;
    let opposite_corner_count = grid.bin(9, 0).row_count;

    assert!(cluster_bin_count > opposite_corner_count);
}

#[test]
fn injected_timeline_patterns_are_visible_in_expected_bins() {
    let dataset = generate_synthetic_events(SyntheticEventConfig::new(33, 400));
    let grid = timeline_density(
        &dataset.events,
        dataset.time_range,
        dataset.lane_count,
        50,
        dataset.lane_count,
    );

    let stale_lane_y_bin = dataset.lane_count - 1;
    let late_time_x_bin = 40;

    let spike_total = sum_timeline_bins(&grid, 23..=25, dataset.lane_count);
    let before_spike_total = sum_timeline_bins(&grid, 20..=22, dataset.lane_count);
    let gap_total = sum_timeline_bins(&grid, 11..=13, dataset.lane_count);
    let stale_lane_late_total: u32 = grid.bin(late_time_x_bin, stale_lane_y_bin).row_count;

    assert!(spike_total > before_spike_total);
    assert_eq!(gap_total, 0);
    assert_eq!(stale_lane_late_total, 0);
}
