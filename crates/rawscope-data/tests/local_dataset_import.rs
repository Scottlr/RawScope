use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use rawscope_core::RowId;
use rawscope_data::{
    load_scatter_dataset, load_timeline_dataset, DatasetChunkId, DatasetLoadError,
    LoadedColumnKind, ScatterPointKind, TimelineEventKind,
};

#[test]
fn csv_scatter_loads_numeric_columns() {
    let path = write_csv_fixture(
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
    let path = write_csv_fixture(
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
    let path = write_csv_fixture("missing_column", "latency_ms\n10\n");

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
    let path = write_csv_fixture(
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
    let path = write_csv_fixture(
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
fn parquet_scatter_loads_numeric_columns_and_preserves_chunk_row_ids() {
    let row_count = 1_025usize;
    let x_values = (0..row_count)
        .map(|row_index| row_index as f64 + 0.5)
        .collect::<Vec<_>>();
    let y_values = (0..row_count)
        .map(|row_index| (row_index as i64) * 2)
        .collect::<Vec<_>>();
    let labels = (0..row_count)
        .map(|row_index| format!("label-{row_index}"))
        .collect::<Vec<_>>();
    let batch = scatter_batch(&x_values, &y_values, &labels);
    let path = write_parquet_fixture("scatter_numeric_parquet", vec![batch]);

    let dataset = load_scatter_dataset(&path, "latency_ms", "payload_size", None).unwrap();
    let columnar = dataset
        .columnar
        .as_ref()
        .expect("Parquet scatter should retain columnar chunk metadata");

    assert_eq!(dataset.points.len(), row_count);
    assert_eq!(dataset.points[0].row_id, RowId(0));
    assert_eq!(dataset.points[1_024].row_id, RowId(1_024));
    assert_eq!(dataset.points[1_024].x, 1_024.5);
    assert_eq!(dataset.points[1_024].y, 2_048.0);
    assert_eq!(
        columnar.chunks,
        vec![
            rawscope_data::LoadedColumnarChunk {
                chunk_id: DatasetChunkId(0),
                row_id_start: RowId(0),
                row_count: 1_024,
            },
            rawscope_data::LoadedColumnarChunk {
                chunk_id: DatasetChunkId(1),
                row_id_start: RowId(1_024),
                row_count: 1,
            },
        ]
    );
    assert_eq!(
        dataset
            .source_rows
            .row(RowId(1_024))
            .expect("last source row should exist")
            .values,
        vec!["1024.5", "2048", "label-1024"]
    );
    remove_fixture(&path);
}

#[test]
fn parquet_timeline_loads_integer_timestamp_and_lane_columns() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::Int64, false),
        Field::new("provider", DataType::UInt64, false),
        Field::new("status", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(Int64Array::from(vec![100_i64, 125, 150])) as ArrayRef,
            Arc::new(UInt64Array::from(vec![7_u64, 9, 7])) as ArrayRef,
            Arc::new(StringArray::from(vec!["ok", "late", "ok"])) as ArrayRef,
        ],
    )
    .unwrap();
    let path = write_parquet_fixture("timeline_integer_lanes_parquet", vec![batch]);

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();
    let columnar = dataset
        .columnar
        .as_ref()
        .expect("Parquet timeline should retain columnar chunk metadata");

    assert_eq!(dataset.events.len(), 3);
    assert_eq!(dataset.time_range.min, 100);
    assert_eq!(dataset.time_range.max, 150);
    assert_eq!(dataset.lane_count, 2);
    assert_eq!(dataset.lane_labels, ["7", "9"]);
    assert_eq!(
        dataset
            .events
            .iter()
            .map(|event| event.lane)
            .collect::<Vec<_>>(),
        vec![0, 1, 0]
    );
    assert_eq!(columnar.chunks.len(), 1);
    assert_eq!(
        dataset
            .source_rows
            .row(RowId(1))
            .expect("second source row should exist")
            .values,
        vec!["125", "9", "late"]
    );
    remove_fixture(&path);
}

#[test]
fn parquet_rejects_unsupported_scatter_binding_type_with_context() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("latency_ms", DataType::Utf8, false),
        Field::new("payload_size", DataType::Int64, false),
    ]));
    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(StringArray::from(vec!["fast"])) as ArrayRef,
            Arc::new(Int64Array::from(vec![10_i64])) as ArrayRef,
        ],
    )
    .unwrap();
    let path = write_parquet_fixture("scatter_unsupported_parquet", vec![batch]);

    let err = load_scatter_dataset(&path, "latency_ms", "payload_size", None)
        .expect_err("string Parquet x column should fail for scatter");

    assert!(matches!(
        err,
        DatasetLoadError::UnsupportedParquetColumnType {
            ref column,
            expected,
            ref actual,
        } if column == "latency_ms"
            && expected == "integer or float-compatible numeric values"
            && actual == "Utf8"
    ));
    remove_fixture(&path);
}

#[test]
fn csv_scatter_retains_source_rows_by_row_id() {
    let path = write_csv_fixture(
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
    let path = write_csv_fixture(
        "timeline_source_rows",
        "timestamp,provider,status\n100,aws,ok\n125,gcp,late\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();
    let second_row = dataset
        .source_rows
        .row(dataset.events[1].row_id)
        .expect("second retained row should exist");

    assert_eq!(second_row.values, vec!["125", "gcp", "late"]);
    assert_eq!(dataset.source_rows.row(RowId(5)), None);
    remove_fixture(&path);
}

#[test]
fn csv_limit_limits_retained_source_rows() {
    let path = write_csv_fixture(
        "scatter_limit_source_rows",
        "latency_ms,payload_size\n1,10\n2,20\n3,30\n",
    );

    let dataset = load_scatter_dataset(&path, "latency_ms", "payload_size", Some(2)).unwrap();

    assert_eq!(dataset.points.len(), 2);
    assert_eq!(dataset.source_rows.rows.len(), 2);
    assert_eq!(
        dataset.source_rows.row(RowId(0)).unwrap().values,
        vec!["1", "10"]
    );
    assert_eq!(
        dataset.source_rows.row(RowId(1)).unwrap().values,
        vec!["2", "20"]
    );
    assert_eq!(dataset.source_rows.row(RowId(2)), None);
    remove_fixture(&path);
}

#[test]
fn csv_zero_limit_is_rejected_before_reading_rows() {
    let path = write_csv_fixture(
        "scatter_zero_limit",
        "latency_ms,payload_size\n1,10\n2,20\n",
    );

    let error = load_scatter_dataset(&path, "latency_ms", "payload_size", Some(0))
        .expect_err("zero CSV limit must be invalid");
    assert!(matches!(
        error,
        DatasetLoadError::InvalidRowLimit { limit: 0 }
    ));
    remove_fixture(&path);
}

#[test]
fn csv_timestamp_above_i64_max_remains_an_unsigned_integer() {
    let path = write_csv_fixture(
        "timeline_unsigned_timestamp",
        "timestamp,provider\n9223372036854775808,aws\n9223372036854775810,gcp\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();

    assert_eq!(dataset.events[0].timestamp, 9_223_372_036_854_775_808);
    assert_eq!(dataset.events[1].timestamp, 9_223_372_036_854_775_810);
    assert_eq!(
        dataset
            .schema
            .iter()
            .find(|column| column.name == "timestamp")
            .map(|column| column.kind),
        Some(LoadedColumnKind::Integer)
    );
    remove_fixture(&path);
}

#[test]
fn source_table_preserves_empty_cell_as_empty_string() {
    let path = write_csv_fixture(
        "timeline_empty_source_cell",
        "timestamp,provider,status\n100,aws,\n",
    );

    let dataset = load_timeline_dataset(&path, "timestamp", "provider", None).unwrap();

    assert_eq!(
        dataset.source_rows.row(RowId(0)).unwrap().values,
        vec!["100", "aws", ""]
    );
    remove_fixture(&path);
}

fn scatter_batch(x_values: &[f64], y_values: &[i64], labels: &[String]) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("latency_ms", DataType::Float64, false),
        Field::new("payload_size", DataType::Int64, false),
        Field::new("label", DataType::Utf8, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Float64Array::from(x_values.to_vec())) as ArrayRef,
            Arc::new(Int64Array::from(y_values.to_vec())) as ArrayRef,
            Arc::new(StringArray::from(labels.to_vec())) as ArrayRef,
        ],
    )
    .unwrap()
}

fn write_csv_fixture(name: &str, contents: &str) -> PathBuf {
    let path = fixture_path_with_extension(name, "csv");
    fs::write(&path, contents).unwrap();
    path
}

fn write_parquet_fixture(name: &str, batches: Vec<RecordBatch>) -> PathBuf {
    let path = fixture_path_with_extension(name, "parquet");
    let schema = batches
        .first()
        .expect("Parquet fixture should include at least one batch")
        .schema();
    let file = File::create(&path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();

    for batch in batches {
        writer.write(&batch).unwrap();
    }

    writer.close().unwrap();
    path
}

fn remove_fixture(path: &Path) {
    fs::remove_file(path).unwrap();
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
