use std::{
    fs::{self, File},
    hint::black_box,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use parquet::arrow::ArrowWriter;
use rawscope_data::{load_scatter_dataset, load_timeline_dataset};

const INGEST_ROW_COUNTS: [usize; 2] = [20_000, 100_000];
const BENCHMARK_TIME_COLUMN: &str = "timestamp";
const BENCHMARK_LANE_COLUMN: &str = "provider";
const BENCHMARK_SCATTER_X_COLUMN: &str = "latency_ms";
const BENCHMARK_SCATTER_Y_COLUMN: &str = "payload_size";

fn local_ingest_benchmarks(c: &mut Criterion) {
    let mut csv_scatter_group = c.benchmark_group("csv_scatter_load");
    csv_scatter_group.sample_size(10);
    csv_scatter_group.warm_up_time(Duration::from_millis(500));
    csv_scatter_group.measurement_time(Duration::from_secs(2));
    for row_count in INGEST_ROW_COUNTS {
        let path = write_csv_fixture(
            &format!("scatter-{row_count}"),
            &scatter_csv_contents(row_count),
        );
        csv_scatter_group.throughput(Throughput::Elements(row_count as u64));
        csv_scatter_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &path,
            |bencher, path| {
                bencher.iter(|| {
                    let dataset = load_scatter_dataset(
                        black_box(path),
                        BENCHMARK_SCATTER_X_COLUMN,
                        BENCHMARK_SCATTER_Y_COLUMN,
                        None,
                    )
                    .expect("CSV scatter benchmark fixture should load");
                    black_box(dataset.points.len())
                });
            },
        );
    }
    csv_scatter_group.finish();

    let mut parquet_scatter_group = c.benchmark_group("parquet_scatter_load");
    parquet_scatter_group.sample_size(10);
    parquet_scatter_group.warm_up_time(Duration::from_millis(500));
    parquet_scatter_group.measurement_time(Duration::from_secs(2));
    for row_count in INGEST_ROW_COUNTS {
        let path = write_parquet_fixture(
            &format!("scatter-{row_count}"),
            scatter_record_batch(row_count),
        );
        parquet_scatter_group.throughput(Throughput::Elements(row_count as u64));
        parquet_scatter_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &path,
            |bencher, path| {
                bencher.iter(|| {
                    let dataset = load_scatter_dataset(
                        black_box(path),
                        BENCHMARK_SCATTER_X_COLUMN,
                        BENCHMARK_SCATTER_Y_COLUMN,
                        None,
                    )
                    .expect("Parquet scatter benchmark fixture should load");
                    black_box(dataset.points.len())
                });
            },
        );
    }
    parquet_scatter_group.finish();

    let mut csv_timeline_group = c.benchmark_group("csv_timeline_load");
    csv_timeline_group.sample_size(10);
    csv_timeline_group.warm_up_time(Duration::from_millis(500));
    csv_timeline_group.measurement_time(Duration::from_secs(2));
    for row_count in INGEST_ROW_COUNTS {
        let path = write_csv_fixture(
            &format!("timeline-{row_count}"),
            &timeline_csv_contents(row_count),
        );
        csv_timeline_group.throughput(Throughput::Elements(row_count as u64));
        csv_timeline_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &path,
            |bencher, path| {
                bencher.iter(|| {
                    let dataset = load_timeline_dataset(
                        black_box(path),
                        BENCHMARK_TIME_COLUMN,
                        BENCHMARK_LANE_COLUMN,
                        None,
                    )
                    .expect("CSV timeline benchmark fixture should load");
                    black_box(dataset.events.len())
                });
            },
        );
    }
    csv_timeline_group.finish();

    let mut parquet_timeline_group = c.benchmark_group("parquet_timeline_load");
    parquet_timeline_group.sample_size(10);
    parquet_timeline_group.warm_up_time(Duration::from_millis(500));
    parquet_timeline_group.measurement_time(Duration::from_secs(2));
    for row_count in INGEST_ROW_COUNTS {
        let path = write_parquet_fixture(
            &format!("timeline-{row_count}"),
            timeline_record_batch(row_count),
        );
        parquet_timeline_group.throughput(Throughput::Elements(row_count as u64));
        parquet_timeline_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &path,
            |bencher, path| {
                bencher.iter(|| {
                    let dataset = load_timeline_dataset(
                        black_box(path),
                        BENCHMARK_TIME_COLUMN,
                        BENCHMARK_LANE_COLUMN,
                        None,
                    )
                    .expect("Parquet timeline benchmark fixture should load");
                    black_box(dataset.events.len())
                });
            },
        );
    }
    parquet_timeline_group.finish();
}

fn scatter_csv_contents(row_count: usize) -> String {
    let mut contents = String::from("latency_ms,payload_size,label\n");
    for row_index in 0..row_count {
        contents.push_str(&format!(
            "{:.3},{},label-{}\n",
            row_index as f64 * 0.25 + 10.0,
            row_index + 512,
            row_index % 8
        ));
    }
    contents
}

fn timeline_csv_contents(row_count: usize) -> String {
    let mut contents = String::from("timestamp,provider,status\n");
    for row_index in 0..row_count {
        contents.push_str(&format!(
            "{},provider-{},ok\n",
            row_index as u64 + 1_000,
            row_index % 16
        ));
    }
    contents
}

fn scatter_record_batch(row_count: usize) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new(BENCHMARK_SCATTER_X_COLUMN, DataType::Float64, false),
        Field::new(BENCHMARK_SCATTER_Y_COLUMN, DataType::Int64, false),
        Field::new("label", DataType::Utf8, false),
    ]));
    let x_values = (0..row_count)
        .map(|row_index| row_index as f64 * 0.25 + 10.0)
        .collect::<Vec<_>>();
    let y_values = (0..row_count)
        .map(|row_index| row_index as i64 + 512)
        .collect::<Vec<_>>();
    let labels = (0..row_count)
        .map(|row_index| format!("label-{}", row_index % 8))
        .collect::<Vec<_>>();

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Float64Array::from(x_values)) as ArrayRef,
            Arc::new(Int64Array::from(y_values)) as ArrayRef,
            Arc::new(StringArray::from(labels)) as ArrayRef,
        ],
    )
    .expect("scatter benchmark record batch should be valid")
}

fn timeline_record_batch(row_count: usize) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new(BENCHMARK_TIME_COLUMN, DataType::Int64, false),
        Field::new(BENCHMARK_LANE_COLUMN, DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
    ]));
    let timestamps = (0..row_count)
        .map(|row_index| row_index as i64 + 1_000)
        .collect::<Vec<_>>();
    let providers = (0..row_count)
        .map(|row_index| format!("provider-{}", row_index % 16))
        .collect::<Vec<_>>();
    let statuses = vec!["ok".to_string(); row_count];

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(timestamps)) as ArrayRef,
            Arc::new(StringArray::from(providers)) as ArrayRef,
            Arc::new(StringArray::from(statuses)) as ArrayRef,
        ],
    )
    .expect("timeline benchmark record batch should be valid")
}

fn write_csv_fixture(name: &str, contents: &str) -> BenchmarkFixture {
    let path = fixture_path(name, "csv");
    fs::write(&path, contents).expect("CSV benchmark fixture should be written");
    BenchmarkFixture::new(path)
}

fn write_parquet_fixture(name: &str, batch: RecordBatch) -> BenchmarkFixture {
    let path = fixture_path(name, "parquet");
    let file = File::create(&path).expect("Parquet benchmark fixture should be created");
    let schema = batch.schema();
    let mut writer =
        ArrowWriter::try_new(file, schema, None).expect("Parquet writer should be created");
    writer
        .write(&batch)
        .expect("Parquet benchmark batch should be written");
    writer.close().expect("Parquet writer should close cleanly");
    BenchmarkFixture::new(path)
}

fn fixture_path(name: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rawscope-bench-{name}-{}.{}",
        std::process::id(),
        extension
    ));
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    path
}

struct BenchmarkFixture {
    path: PathBuf,
}

impl BenchmarkFixture {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl AsRef<Path> for BenchmarkFixture {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for BenchmarkFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

criterion_group!(benches, local_ingest_benchmarks);
criterion_main!(benches);
