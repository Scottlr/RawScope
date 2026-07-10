use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{
    LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
    ScatterPointRecord, SyntheticEventType, SyntheticPointCategory, TimelineEventKind,
    TimelineEventRecord,
};
use rawscope_render::{
    missingness_selection_comparison, missingness_selection_summary, scatter_selection_comparison,
    timeline_selection_comparison, ComparisonRatio, MissingnessSelection,
    MissingnessSelectionSummary, ScatterBrushSelection, SelectedRegionSummary,
    TimelineBrushSelection, TimelineLaneRange, TimelineSelectionSummary,
};

fn scatter_viewport_selection() -> ScatterBrushSelection {
    ScatterBrushSelection {
        x_range: F32Range::new(0.0, 50.0),
        y_range: F32Range::new(0.0, 50.0),
    }
}

fn scatter_point(row_id: u64, x: f32, y: f32, kind: ScatterPointKind) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind,
    }
}

fn timeline_selection() -> TimelineBrushSelection {
    TimelineBrushSelection {
        time_range: U64Range::new(0, 25),
        lane_range: TimelineLaneRange::new(0, 2),
    }
}

fn timeline_event(
    row_id: u64,
    timestamp: u64,
    lane: u32,
    value: f32,
    kind: TimelineEventKind,
) -> TimelineEventRecord {
    TimelineEventRecord {
        row_id: RowId(row_id),
        timestamp,
        lane,
        value,
        kind,
    }
}

fn source_table_fixture() -> LoadedSourceTable {
    LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "alpha".to_string(),
                kind: LoadedColumnKind::String,
            },
            LoadedColumnSchema {
                name: "beta".to_string(),
                kind: LoadedColumnKind::String,
            },
            LoadedColumnSchema {
                name: "gamma".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["".to_string(), "  ".to_string(), "ok".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["1".to_string(), "".to_string(), "ok".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["2".to_string(), "value".to_string(), " ".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(3),
                values: vec!["3".to_string(), "value".to_string(), "ok".to_string()],
            },
        ],
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

fn assert_ratio(
    ratio: &ComparisonRatio,
    selected_count: usize,
    baseline_count: usize,
    selected_percentage: f32,
    baseline_percentage: f32,
    delta_percentage_points: f32,
) {
    assert_eq!(ratio.selected_count, selected_count);
    assert_eq!(ratio.baseline_count, baseline_count);
    assert_close(ratio.selected_percentage, selected_percentage);
    assert_close(ratio.baseline_percentage, baseline_percentage);
    assert_close(ratio.delta_percentage_points, delta_percentage_points);
}

#[test]
fn scatter_comparison_reports_selected_vs_baseline_kind_share() {
    let points = vec![
        scatter_point(
            0,
            10.0,
            10.0,
            ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
        ),
        scatter_point(
            1,
            20.0,
            20.0,
            ScatterPointKind::Synthetic(SyntheticPointCategory::Background),
        ),
        scatter_point(
            2,
            80.0,
            80.0,
            ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier),
        ),
        scatter_point(3, 90.0, 10.0, ScatterPointKind::Unclassified),
    ];
    let summary = SelectedRegionSummary::from_points(&points, scatter_viewport_selection());

    let comparison = scatter_selection_comparison(&points, summary);

    assert_eq!(comparison.selected_row_count, 2);
    assert_eq!(comparison.baseline_row_count, 4);
    assert_close(comparison.selected_percentage, 50.0);
    assert_ratio(
        &comparison.point_kind_ratios.cluster,
        1,
        1,
        50.0,
        25.0,
        25.0,
    );
    assert_ratio(
        &comparison.point_kind_ratios.background,
        1,
        1,
        50.0,
        25.0,
        25.0,
    );
    assert_ratio(
        &comparison.point_kind_ratios.outlier,
        0,
        1,
        0.0,
        25.0,
        -25.0,
    );
    assert_ratio(
        &comparison.point_kind_ratios.unclassified,
        0,
        1,
        0.0,
        25.0,
        -25.0,
    );
}

#[test]
fn timeline_comparison_reports_lane_share_delta() {
    let events = vec![
        timeline_event(
            0,
            10,
            0,
            1.0,
            TimelineEventKind::Synthetic(SyntheticEventType::Background),
        ),
        timeline_event(
            1,
            20,
            1,
            2.0,
            TimelineEventKind::Synthetic(SyntheticEventType::Spike),
        ),
        timeline_event(
            2,
            30,
            2,
            3.0,
            TimelineEventKind::Synthetic(SyntheticEventType::StaleLane),
        ),
        timeline_event(3, 40, 2, 4.0, TimelineEventKind::Unclassified),
    ];
    let summary = TimelineSelectionSummary::from_events(&events, timeline_selection(), 3);

    let comparison = timeline_selection_comparison(&events, 3, &summary);

    assert_eq!(comparison.selected_event_count, 2);
    assert_eq!(comparison.baseline_event_count, 4);
    assert_close(comparison.selected_percentage, 50.0);
    assert_ratio(
        &comparison.event_kind_ratios.background,
        1,
        1,
        50.0,
        25.0,
        25.0,
    );
    assert_ratio(&comparison.event_kind_ratios.spike, 1, 1, 50.0, 25.0, 25.0);
    assert_ratio(
        &comparison.event_kind_ratios.stale_lane,
        0,
        1,
        0.0,
        25.0,
        -25.0,
    );
    assert_ratio(
        &comparison.event_kind_ratios.high_value_band,
        0,
        0,
        0.0,
        0.0,
        0.0,
    );
    assert_ratio(
        &comparison.event_kind_ratios.unclassified,
        0,
        1,
        0.0,
        25.0,
        -25.0,
    );
    assert_eq!(comparison.lane_ratios.len(), 3);
    assert_ratio(&comparison.lane_ratios[0], 1, 1, 50.0, 25.0, 25.0);
    assert_ratio(&comparison.lane_ratios[1], 1, 1, 50.0, 25.0, 25.0);
    assert_ratio(&comparison.lane_ratios[2], 0, 2, 0.0, 50.0, -50.0);
}

#[test]
fn missingness_comparison_reports_ratio_delta() {
    let source_table = source_table_fixture();
    let selected =
        missingness_selection_summary(&source_table, 2, MissingnessSelection::new(0, 1, 1, 2));
    let baseline_missing_count = count_missing_values(&source_table);
    let baseline_total_count = total_cell_count(&source_table);

    let comparison =
        missingness_selection_comparison(&selected, baseline_missing_count, baseline_total_count);

    assert_eq!(comparison.selected_missing_count, 2);
    assert_eq!(comparison.selected_total_count, 2);
    assert_eq!(comparison.baseline_missing_count, 4);
    assert_eq!(comparison.baseline_total_count, 12);
    assert_close(comparison.selected_missing_ratio, 1.0);
    assert_close(comparison.baseline_missing_ratio, 0.33333334);
    assert_close(comparison.delta_percentage_points, 66.66666);
}

#[test]
fn comparison_ratios_do_not_nan_on_empty_baseline() {
    let empty_scatter_points: Vec<ScatterPointRecord> = Vec::new();
    let scatter_summary = SelectedRegionSummary::from_points(
        &empty_scatter_points,
        ScatterBrushSelection {
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
        },
    );
    let scatter_comparison = scatter_selection_comparison(&empty_scatter_points, scatter_summary);

    assert_zero_ratio(&scatter_comparison.point_kind_ratios.cluster);
    assert_zero_ratio(&scatter_comparison.point_kind_ratios.background);
    assert_zero_ratio(&scatter_comparison.point_kind_ratios.outlier);
    assert_zero_ratio(&scatter_comparison.point_kind_ratios.unclassified);

    let empty_events: Vec<TimelineEventRecord> = Vec::new();
    let timeline_summary = TimelineSelectionSummary::from_events(
        &empty_events,
        TimelineBrushSelection {
            time_range: U64Range::new(0, 1),
            lane_range: TimelineLaneRange::new(0, 1),
        },
        2,
    );
    let timeline_comparison = timeline_selection_comparison(&empty_events, 2, &timeline_summary);

    assert_eq!(timeline_comparison.lane_ratios.len(), 2);
    assert_zero_ratio(&timeline_comparison.event_kind_ratios.background);
    assert_zero_ratio(&timeline_comparison.event_kind_ratios.spike);
    assert_zero_ratio(&timeline_comparison.event_kind_ratios.stale_lane);
    assert_zero_ratio(&timeline_comparison.event_kind_ratios.high_value_band);
    assert_zero_ratio(&timeline_comparison.event_kind_ratios.unclassified);
    assert_zero_ratio(&timeline_comparison.lane_ratios[0]);
    assert_zero_ratio(&timeline_comparison.lane_ratios[1]);

    let empty_missingness = MissingnessSelectionSummary {
        selected_missing_count: 0,
        selected_total_count: 0,
        selected_row_ids: Vec::new(),
        column_names: Vec::new(),
    };
    let missingness_comparison = missingness_selection_comparison(&empty_missingness, 0, 0);

    assert_eq!(missingness_comparison.selected_missing_ratio, 0.0);
    assert_eq!(missingness_comparison.baseline_missing_ratio, 0.0);
    assert_eq!(missingness_comparison.delta_percentage_points, 0.0);
    assert!(missingness_comparison.selected_missing_ratio.is_finite());
    assert!(missingness_comparison.baseline_missing_ratio.is_finite());
    assert!(missingness_comparison.delta_percentage_points.is_finite());
}

fn count_missing_values(table: &LoadedSourceTable) -> u64 {
    table
        .rows
        .iter()
        .flat_map(|row| row.values.iter())
        .filter(|value| value.trim().is_empty())
        .count() as u64
}

fn total_cell_count(table: &LoadedSourceTable) -> u64 {
    (table.rows.len() * table.columns.len()) as u64
}

fn assert_zero_ratio(ratio: &ComparisonRatio) {
    assert_eq!(ratio.selected_count, 0);
    assert_eq!(ratio.baseline_count, 0);
    assert_eq!(ratio.selected_percentage, 0.0);
    assert_eq!(ratio.baseline_percentage, 0.0);
    assert_eq!(ratio.delta_percentage_points, 0.0);
    assert!(ratio.selected_percentage.is_finite());
    assert!(ratio.baseline_percentage.is_finite());
    assert!(ratio.delta_percentage_points.is_finite());
}
