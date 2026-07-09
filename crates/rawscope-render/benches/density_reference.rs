use std::{hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rawscope_data::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticPointConfig,
};
use rawscope_render::{scatter_density, timeline_density};

const DENSITY_GRID_WIDTH: u32 = 512;
const DENSITY_GRID_HEIGHT: u32 = 512;
const BENCHMARK_SEED: u64 = 42;
const SCATTER_ROW_COUNTS: [usize; 2] = [200_000, 1_000_000];
const TIMELINE_ROW_COUNTS: [usize; 2] = [200_000, 1_000_000];

fn density_reference_benchmarks(c: &mut Criterion) {
    let mut scatter_group = c.benchmark_group("scatter_density_reference");
    scatter_group.sample_size(10);
    scatter_group.warm_up_time(Duration::from_millis(500));
    scatter_group.measurement_time(Duration::from_secs(2));
    for row_count in SCATTER_ROW_COUNTS {
        let dataset =
            generate_synthetic_points(SyntheticPointConfig::new(BENCHMARK_SEED, row_count));
        scatter_group.throughput(Throughput::Elements(row_count as u64));
        scatter_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    let grid = scatter_density(
                        black_box(&dataset.points),
                        dataset.x_range,
                        dataset.y_range,
                        DENSITY_GRID_WIDTH,
                        DENSITY_GRID_HEIGHT,
                    );
                    black_box(grid.total_row_count())
                });
            },
        );
    }
    scatter_group.finish();

    let mut timeline_group = c.benchmark_group("timeline_density_reference");
    timeline_group.sample_size(10);
    timeline_group.warm_up_time(Duration::from_millis(500));
    timeline_group.measurement_time(Duration::from_secs(2));
    for row_count in TIMELINE_ROW_COUNTS {
        let dataset =
            generate_synthetic_events(SyntheticEventConfig::new(BENCHMARK_SEED, row_count));
        timeline_group.throughput(Throughput::Elements(row_count as u64));
        timeline_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    let grid = timeline_density(
                        black_box(&dataset.events),
                        dataset.time_range,
                        dataset.lane_count,
                        DENSITY_GRID_WIDTH,
                        DENSITY_GRID_HEIGHT,
                    );
                    black_box(grid.total_row_count())
                });
            },
        );
    }
    timeline_group.finish();
}

criterion_group!(benches, density_reference_benchmarks);
criterion_main!(benches);
