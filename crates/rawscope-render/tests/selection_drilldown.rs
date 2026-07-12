use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{
    LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
    ScatterPointRecord, SyntheticPointCategory, TimelineEventKind, TimelineEventRecord,
};
use rawscope_render::{
    scatter_selection_drilldown, timeline_selection_drilldown, DrilldownConfig,
    ScatterBrushSelection, TimelineBrushSelection, TimelineLaneRange,
};

fn scatter_selection() -> ScatterBrushSelection {
    ScatterBrushSelection {
        x_range: F32Range::new(0.0, 50.0),
        y_range: F32Range::new(0.0, 50.0),
    }
}

fn timeline_selection() -> TimelineBrushSelection {
    TimelineBrushSelection {
        time_range: U64Range::new(100, 300),
        lane_range: TimelineLaneRange::new(0, 2),
    }
}

#[test]
fn scatter_drilldown_uses_lowest_selected_row_ids_from_source_rows() {
    let points = vec![
        ScatterPointRecord {
            row_id: RowId(9),
            x: 15.0,
            y: 25.0,
            kind: ScatterPointKind::Unclassified,
        },
        ScatterPointRecord {
            row_id: RowId(2),
            x: 10.0,
            y: 20.0,
            kind: ScatterPointKind::Unclassified,
        },
        ScatterPointRecord {
            row_id: RowId(5),
            x: 30.0,
            y: 40.0,
            kind: ScatterPointKind::Unclassified,
        },
    ];
    let source_rows = LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "x".to_string(),
                kind: LoadedColumnKind::Float,
            },
            LoadedColumnSchema {
                name: "label".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["0".to_string(), "zero".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["1".to_string(), "one".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["10".to_string(), "alpha".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(3),
                values: vec!["3".to_string(), "three".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(4),
                values: vec!["4".to_string(), "four".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(5),
                values: vec!["30".to_string(), "beta".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(6),
                values: vec!["6".to_string(), "six".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(7),
                values: vec!["7".to_string(), "seven".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(8),
                values: vec!["8".to_string(), "eight".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(9),
                values: vec!["15".to_string(), "gamma".to_string()],
            },
        ],
    };

    let drilldown = scatter_selection_drilldown(
        &points,
        scatter_selection(),
        Some(&source_rows),
        DrilldownConfig { max_rows: 2 },
    );

    assert_eq!(drilldown.selected_row_count, 3);
    assert_eq!(drilldown.displayed_row_count, 2);
    assert!(drilldown.rows_are_sampled);
    assert_eq!(drilldown.columns[0].name, "x");
    assert_eq!(drilldown.rows[0].row_id, RowId(2));
    assert_eq!(drilldown.rows[0].values, vec!["10", "alpha"]);
    assert_eq!(drilldown.rows[1].row_id, RowId(5));
    assert!(drilldown.completeness.sampled_due_to_limit());
    assert!(drilldown.completeness.unavailable().is_empty());
}

#[test]
fn scatter_drilldown_uses_visual_fallback_without_source_rows() {
    let points = vec![
        ScatterPointRecord {
            row_id: RowId(4),
            x: 12.5,
            y: 22.0,
            kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier),
        },
        ScatterPointRecord {
            row_id: RowId(1),
            x: 10.0,
            y: 20.0,
            kind: ScatterPointKind::Unclassified,
        },
    ];

    let drilldown = scatter_selection_drilldown(
        &points,
        scatter_selection(),
        None,
        DrilldownConfig { max_rows: 10 },
    );

    assert_eq!(
        drilldown
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["row_id", "x", "y", "kind"]
    );
    assert_eq!(drilldown.rows[0].row_id, RowId(1));
    assert_eq!(
        drilldown.rows[0].values,
        vec![
            "1".to_string(),
            "10.000000".to_string(),
            "20.000000".to_string(),
            "unclassified".to_string(),
        ]
    );
    assert_eq!(drilldown.rows[1].values[3], "outlier");
}

#[test]
fn timeline_drilldown_uses_source_rows_when_available() {
    let events = vec![
        TimelineEventRecord {
            row_id: RowId(0),
            timestamp: 120,
            lane: 0,
            value: 1.0,
            kind: TimelineEventKind::Unclassified,
        },
        TimelineEventRecord {
            row_id: RowId(3),
            timestamp: 150,
            lane: 1,
            value: 2.0,
            kind: TimelineEventKind::Unclassified,
        },
        TimelineEventRecord {
            row_id: RowId(5),
            timestamp: 320,
            lane: 1,
            value: 3.0,
            kind: TimelineEventKind::Unclassified,
        },
    ];
    let source_rows = LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "timestamp".to_string(),
                kind: LoadedColumnKind::Integer,
            },
            LoadedColumnSchema {
                name: "provider".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: vec![
            LoadedSourceRow {
                row_id: RowId(0),
                values: vec!["120".to_string(), "aws".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(1),
                values: vec!["130".to_string(), "gcp".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(2),
                values: vec!["140".to_string(), "azure".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(3),
                values: vec!["150".to_string(), "gcp".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(4),
                values: vec!["160".to_string(), "aws".to_string()],
            },
            LoadedSourceRow {
                row_id: RowId(5),
                values: vec!["320".to_string(), "tail".to_string()],
            },
        ],
    };

    let drilldown = timeline_selection_drilldown(
        &events,
        timeline_selection(),
        Some(&source_rows),
        DrilldownConfig { max_rows: 10 },
    );

    assert_eq!(drilldown.selected_row_count, 2);
    assert_eq!(drilldown.displayed_row_count, 2);
    assert!(!drilldown.rows_are_sampled);
    assert_eq!(drilldown.columns[1].name, "provider");
    assert_eq!(drilldown.rows[0].row_id, RowId(0));
    assert_eq!(drilldown.rows[1].values, vec!["150", "gcp"]);
}

#[test]
fn timeline_drilldown_uses_visual_fallback_and_respects_max_rows() {
    let events = vec![
        TimelineEventRecord {
            row_id: RowId(7),
            timestamp: 140,
            lane: 1,
            value: 7.0,
            kind: TimelineEventKind::Unclassified,
        },
        TimelineEventRecord {
            row_id: RowId(1),
            timestamp: 110,
            lane: 0,
            value: 1.5,
            kind: TimelineEventKind::Unclassified,
        },
        TimelineEventRecord {
            row_id: RowId(3),
            timestamp: 180,
            lane: 1,
            value: 3.5,
            kind: TimelineEventKind::Unclassified,
        },
    ];

    let drilldown = timeline_selection_drilldown(
        &events,
        timeline_selection(),
        None,
        DrilldownConfig { max_rows: 2 },
    );

    assert_eq!(drilldown.selected_row_count, 3);
    assert_eq!(drilldown.displayed_row_count, 2);
    assert!(drilldown.rows_are_sampled);
    assert_eq!(drilldown.rows[0].row_id, RowId(1));
    assert_eq!(
        drilldown.rows[0].values,
        vec![
            "1".to_string(),
            "110".to_string(),
            "0".to_string(),
            "1.500000".to_string(),
            "unclassified".to_string(),
        ]
    );
}

#[test]
fn empty_selection_produces_zero_row_drilldown() {
    let points = vec![ScatterPointRecord {
        row_id: RowId(0),
        x: 10.0,
        y: 20.0,
        kind: ScatterPointKind::Unclassified,
    }];
    let empty_selection = ScatterBrushSelection {
        x_range: F32Range::new(80.0, 90.0),
        y_range: F32Range::new(80.0, 90.0),
    };

    let drilldown =
        scatter_selection_drilldown(&points, empty_selection, None, DrilldownConfig::default());

    assert_eq!(drilldown.selected_row_count, 0);
    assert_eq!(drilldown.displayed_row_count, 0);
    assert!(!drilldown.rows_are_sampled);
    assert!(drilldown.rows.is_empty());
}
