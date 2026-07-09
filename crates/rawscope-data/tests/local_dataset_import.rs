use std::{
    fs,
    path::{Path, PathBuf},
};

use rawscope_data::{
    load_scatter_dataset, load_timeline_dataset, DatasetLoadError, LoadedColumnKind,
    ScatterPointKind, TimelineEventKind,
};

#[test]
fn csv_scatter_loads_numeric_columns() {
    let path = write_fixture(
        "scatter_numeric",
        "latency_ms,payload_size,label\n10.5,512,a\n20,1024,b\n",
    );

    let dataset = load_scatter_dataset(&path, "latency_ms", "payload_size", None).unwrap();

    assert_eq!(dataset.points.len(), 2);
    assert_eq!(dataset.points[0].row_id.0, 0);
    assert_eq!(dataset.points[0].x, 10.5);
    assert_eq!(dataset.points[0].y, 512.0);
    assert_eq!(dataset.points[0].kind, ScatterPointKind::Unclassified);
    assert_eq!(dataset.source_rows.rows.len(), dataset.points.len());
    assert_eq!(dataset.x_range.min, 10.5);
    assert_eq!(dataset.x_range.max, 20.0);
    assert_eq!(dataset.y_range.min, 512.0);
    assert_eq!(dataset.y_range.max, 1024.0);
    assert_eq!(
        dataset
            .schema
            .iter()
            .find(|column| column.name == "payload_size")
            .map(|column| column.kind),
        Some(LoadedColumnKind::Integer)
    );
    remove_fixture(&path);
}

#[test]
fn csv_timeline_loads_timestamp_and_lane_columns() {
    let path = write_fixture(
        "timeline_lanes",
        "timestamp,provider\n100,aws\n125,gcp\n150,aws\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();

    assert_eq!(dataset.events.len(), 3);
    assert_eq!(dataset.time_range.min, 100);
    assert_eq!(dataset.time_range.max, 150);
    assert_eq!(dataset.lane_count, 2);
    assert_eq!(dataset.lane_labels, ["aws", "gcp"]);
    assert_eq!(dataset.events[0].lane, 0);
    assert_eq!(dataset.events[1].lane, 1);
    assert_eq!(dataset.events[2].lane, 0);
    assert_eq!(dataset.events[0].kind, TimelineEventKind::Unclassified);
    assert_eq!(dataset.source_rows.rows.len(), dataset.events.len());
    remove_fixture(&path);
}

#[test]
fn missing_column_returns_clear_error() {
    let path = write_fixture("missing_column", "latency_ms\n10\n");

    let err = load_scatter_dataset(&path, "latency_ms", "payload_size", None)
        .expect_err("missing y column should fail");

    assert!(matches!(
        err,
        DatasetLoadError::MissingColumn { ref column, .. } if column == "payload_size"
    ));
    assert!(err.to_string().contains("available columns: latency_ms"));
    remove_fixture(&path);
}

#[test]
fn unsupported_scatter_type_returns_clear_error() {
    let path = write_fixture(
        "unsupported_scatter_type",
        "latency_ms,payload_size\nfast,10\n",
    );

    let err = load_scatter_dataset(&path, "latency_ms", "payload_size", None)
        .expect_err("string x column should fail for scatter");

    assert!(matches!(
        err,
        DatasetLoadError::UnsupportedColumnType { ref column, actual, .. }
            if column == "latency_ms" && actual == LoadedColumnKind::String
    ));
    assert!(err.to_string().contains("expected integer or float"));
    remove_fixture(&path);
}

#[test]
fn string_lanes_map_deterministically_by_first_seen_value() {
    let path = write_fixture(
        "deterministic_lanes",
        "timestamp,provider\n1,zeta\n2,alpha\n3,zeta\n4,beta\n",
    );

    let first = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();
    let second = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();

    assert_eq!(first.lane_labels, second.lane_labels);
    assert_eq!(first.lane_labels, ["zeta", "alpha", "beta"]);
    assert_eq!(
        first
            .events
            .iter()
            .map(|event| event.lane)
            .collect::<Vec<_>>(),
        vec![0, 1, 0, 2]
    );
    remove_fixture(&path);
}

#[test]
fn parquet_input_returns_deferred_error() {
    let path = fixture_path_with_extension("parquet_deferred", "parquet");

    let err = load_scatter_dataset(&path, "latency_ms", "payload_size", None)
        .expect_err("Parquet should be explicit deferred for this slice");

    assert!(matches!(err, DatasetLoadError::ParquetDeferred { .. }));
    assert!(err.to_string().contains("Parquet import is not included"));
}

#[test]
fn csv_scatter_retains_source_rows_by_row_id() {
    let path = write_fixture(
        "scatter_source_rows",
        "latency_ms,payload_size,label\n10.5,512,a\n20,1024,b\n",
    );

    let dataset = load_scatter_dataset(&path, "latency_ms", "payload_size", None).unwrap();
    let first_row = dataset
        .source_rows
        .row(dataset.points[0].row_id)
        .expect("first retained row should exist");

    assert_eq!(
        dataset.source_rows.column_names().collect::<Vec<_>>(),
        vec!["latency_ms", "payload_size", "label"]
    );
    assert_eq!(first_row.values, vec!["10.5", "512", "a"]);
    assert_eq!(
        dataset
            .source_rows
            .row(dataset.points[1].row_id)
            .unwrap()
            .values,
        vec!["20", "1024", "b"]
    );
    remove_fixture(&path);
}

#[test]
fn csv_timeline_retains_source_rows_by_row_id() {
    let path = write_fixture(
        "timeline_source_rows",
        "timestamp,provider,status\n100,aws,ok\n125,gcp,late\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();
    let second_row = dataset
        .source_rows
        .row(dataset.events[1].row_id)
        .expect("second retained row should exist");

    assert_eq!(second_row.values, vec!["125", "gcp", "late"]);
    assert_eq!(dataset.source_rows.row(rawscope_core::RowId(5)), None);
    remove_fixture(&path);
}

#[test]
fn csv_limit_limits_retained_source_rows() {
    let path = write_fixture(
        "scatter_limit_source_rows",
        "latency_ms,payload_size\n1,10\n2,20\n3,30\n",
    );

    let dataset = load_scatter_dataset(&path, "latency_ms", "payload_size", Some(2)).unwrap();

    assert_eq!(dataset.points.len(), 2);
    assert_eq!(dataset.source_rows.rows.len(), 2);
    assert_eq!(
        dataset
            .source_rows
            .row(rawscope_core::RowId(0))
            .unwrap()
            .values,
        vec!["1", "10"]
    );
    assert_eq!(
        dataset
            .source_rows
            .row(rawscope_core::RowId(1))
            .unwrap()
            .values,
        vec!["2", "20"]
    );
    assert_eq!(dataset.source_rows.row(rawscope_core::RowId(2)), None);
    remove_fixture(&path);
}

#[test]
fn source_table_preserves_empty_cell_as_empty_string() {
    let path = write_fixture(
        "timeline_empty_source_cell",
        "timestamp,provider,status\n100,aws,\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();

    assert_eq!(
        dataset
            .source_rows
            .row(rawscope_core::RowId(0))
            .unwrap()
            .values,
        vec!["100", "aws", ""]
    );
    remove_fixture(&path);
}

fn write_fixture(name: &str, contents: &str) -> PathBuf {
    let path = fixture_path(name);
    fs::write(&path, contents).unwrap();
    path
}

fn remove_fixture(path: &Path) {
    fs::remove_file(path).unwrap();
}

fn fixture_path(name: &str) -> PathBuf {
    fixture_path_with_extension(name, "csv")
}

fn fixture_path_with_extension(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rawscope-{name}-{}.{}",
        std::process::id(),
        extension
    ));
    if path.exists() {
        fs::remove_file(&path).unwrap();
    }
    path
}
