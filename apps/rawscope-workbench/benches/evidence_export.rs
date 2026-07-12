use std::{
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rawscope_core::{F32Range, RowId, U64Range};
use rawscope_data::{
    DatasetIdentity, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable,
    ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};
use rawscope_render::{
    ScatterBrushSelection, ScatterEvidenceView, ScatterSelectionEvidence,
    ScatterSelectionEvidenceV2, SelectionEvidenceConfig, TimelineBrushSelection,
    TimelineEvidenceConfig, TimelineEvidenceView, TimelineLaneRange, TimelineSelectionEvidence,
    TimelineSelectionEvidenceV2,
};
use rawscope_workbench::benchmark_support::{
    write_scatter_report_bundle, write_timeline_report_bundle, BenchmarkRunMetadata,
};

const EXPORT_BENCHMARK_TIMESTAMP_MS: u128 = 1_735_689_600_000;
const EXPORT_BENCHMARK_ROW_COUNT: usize = 20_000;
const EXPORT_SAMPLE_SIZE: usize = 5;

static EXPORT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn evidence_export_benchmarks(c: &mut Criterion) {
    let output_root = BenchmarkOutputRoot::new();
    fs::create_dir_all(output_root.path()).expect("benchmark output root should be created");
    for metadata in [
        BenchmarkRunMetadata::for_scenario(
            "workbench-evidence-export-scatter-v1",
            "fixture-v1",
            EXPORT_BENCHMARK_ROW_COUNT as u64,
            "workspace-default",
            10,
            "bundle_writes",
        ),
        BenchmarkRunMetadata::for_scenario(
            "workbench-evidence-export-timeline-v1",
            "fixture-v1",
            EXPORT_BENCHMARK_ROW_COUNT as u64,
            "workspace-default",
            10,
            "bundle_writes",
        ),
    ] {
        metadata
            .validate()
            .expect("benchmark metadata fixture should be complete");
        black_box(
            metadata
                .to_json()
                .expect("benchmark metadata should serialize"),
        );
    }

    let scatter_evidence = scatter_evidence_fixture();
    let timeline_evidence = timeline_evidence_fixture();

    let mut group = c.benchmark_group("workbench_evidence_export");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    group.throughput(Throughput::Elements(EXPORT_SAMPLE_SIZE as u64));

    group.bench_with_input(
        BenchmarkId::new("scatter_bundle_write", EXPORT_SAMPLE_SIZE),
        &scatter_evidence,
        |bencher, evidence| {
            bencher.iter(|| {
                let export_counter = EXPORT_COUNTER.fetch_add(1, Ordering::Relaxed);
                write_scatter_report_bundle(
                    output_root.path(),
                    evidence,
                    EXPORT_BENCHMARK_TIMESTAMP_MS,
                    export_counter,
                )
                .expect("scatter benchmark bundle should write");
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("timeline_bundle_write", EXPORT_SAMPLE_SIZE),
        &timeline_evidence,
        |bencher, evidence| {
            bencher.iter(|| {
                let export_counter = EXPORT_COUNTER.fetch_add(1, Ordering::Relaxed);
                write_timeline_report_bundle(
                    output_root.path(),
                    evidence,
                    EXPORT_BENCHMARK_TIMESTAMP_MS,
                    export_counter,
                )
                .expect("timeline benchmark bundle should write");
            });
        },
    );

    group.finish();
}

fn scatter_evidence_fixture() -> ScatterSelectionEvidenceV2 {
    let points = (0..EXPORT_BENCHMARK_ROW_COUNT)
        .map(|row_index| ScatterPointRecord {
            row_id: RowId(row_index as u64),
            x: row_index as f32 * 0.5,
            y: 1_000.0 - row_index as f32 * 0.25,
            kind: ScatterPointKind::Unclassified,
        })
        .collect::<Vec<_>>();
    let selection = ScatterBrushSelection {
        x_range: F32Range::new(0.0, 250.0),
        y_range: F32Range::new(750.0, 1_000.0),
    };
    let source_rows = LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "latency_ms".to_string(),
                kind: LoadedColumnKind::Float,
            },
            LoadedColumnSchema {
                name: "payload_size".to_string(),
                kind: LoadedColumnKind::Integer,
            },
            LoadedColumnSchema {
                name: "label".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: (0..EXPORT_BENCHMARK_ROW_COUNT)
            .map(|row_index| LoadedSourceRow {
                row_id: RowId(row_index as u64),
                values: vec![
                    format!("{:.3}", row_index as f32 * 0.5),
                    (row_index + 512).to_string(),
                    format!("label-{}", row_index % 8),
                ],
            })
            .collect(),
    };
    let v1 = ScatterSelectionEvidence::from_points(
        &points,
        selection,
        rawscope_data::SyntheticDatasetMetadata::new(0, EXPORT_BENCHMARK_ROW_COUNT),
        EXPORT_BENCHMARK_ROW_COUNT,
        SelectionEvidenceConfig {
            max_sample_size: EXPORT_SAMPLE_SIZE,
        },
    );

    ScatterSelectionEvidenceV2::from_v1(
        &v1,
        DatasetIdentity::local_parquet_scatter(
            PathBuf::from("C:/bench/latency.parquet"),
            EXPORT_BENCHMARK_ROW_COUNT,
            Some(EXPORT_BENCHMARK_ROW_COUNT),
            "latency_ms",
            "payload_size",
        ),
        ScatterEvidenceView {
            x_range: F32Range::new(0.0, 1_000.0),
            y_range: F32Range::new(0.0, 1_000.0),
            grid_width: 512,
            grid_height: 512,
        },
        Some(&source_rows),
    )
}

fn timeline_evidence_fixture() -> TimelineSelectionEvidenceV2 {
    let events = (0..EXPORT_BENCHMARK_ROW_COUNT)
        .map(|row_index| TimelineEventRecord {
            row_id: RowId(row_index as u64),
            timestamp: 1_000 + row_index as u64,
            lane: (row_index % 8) as u32,
            value: 1.0 + (row_index % 16) as f32,
            kind: TimelineEventKind::Unclassified,
        })
        .collect::<Vec<_>>();
    let selection = TimelineBrushSelection {
        time_range: U64Range::new(1_000, 1_500),
        lane_range: TimelineLaneRange::new(0, 4),
    };
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
            LoadedColumnSchema {
                name: "status".to_string(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: (0..EXPORT_BENCHMARK_ROW_COUNT)
            .map(|row_index| LoadedSourceRow {
                row_id: RowId(row_index as u64),
                values: vec![
                    (1_000 + row_index as u64).to_string(),
                    format!("provider-{}", row_index % 8),
                    "ok".to_string(),
                ],
            })
            .collect(),
    };
    let v1 = TimelineSelectionEvidence::from_events(
        &events,
        selection,
        8,
        rawscope_data::SyntheticDatasetMetadata::new(0, EXPORT_BENCHMARK_ROW_COUNT),
        EXPORT_BENCHMARK_ROW_COUNT,
        TimelineEvidenceConfig {
            max_sample_size: EXPORT_SAMPLE_SIZE,
        },
    );

    TimelineSelectionEvidenceV2::from_v1(
        &v1,
        DatasetIdentity::local_parquet_timeline(
            PathBuf::from("C:/bench/events.parquet"),
            EXPORT_BENCHMARK_ROW_COUNT,
            Some(EXPORT_BENCHMARK_ROW_COUNT),
            "timestamp",
            "provider",
            (0..8).map(|lane| format!("provider-{lane}")).collect(),
        ),
        TimelineEvidenceView {
            time_range: U64Range::new(1_000, 1_500),
            full_time_range: U64Range::new(1_000, 1_000 + EXPORT_BENCHMARK_ROW_COUNT as u64),
            lane_count: 8,
            grid_width: 512,
            grid_height: 8,
        },
        Some(&source_rows),
    )
}

struct BenchmarkOutputRoot {
    path: PathBuf,
}

impl BenchmarkOutputRoot {
    fn new() -> Self {
        let path = benchmark_output_root();
        if path.exists() {
            let expected_parent = std::env::temp_dir();
            if path.parent() != Some(expected_parent.as_path()) {
                panic!("benchmark output root escaped the process temp directory");
            }
            fs::remove_dir_all(&path).expect("stale benchmark output root should be removable");
        }
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for BenchmarkOutputRoot {
    fn drop(&mut self) {
        if self.path.exists() {
            let expected_parent = std::env::temp_dir();
            if self.path.parent() == Some(expected_parent.as_path()) {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }
}

fn benchmark_output_root() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rawscope-bench-evidence-export-{}",
        std::process::id()
    ));
    path
}

criterion_group!(benches, evidence_export_benchmarks);
criterion_main!(benches);
