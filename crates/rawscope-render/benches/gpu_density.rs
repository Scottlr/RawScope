use std::{env, hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pollster::block_on;
use rawscope_data::{
    generate_synthetic_events, generate_synthetic_points, SyntheticEventConfig,
    SyntheticPointConfig,
};
use rawscope_gpu::ComputeContext;
use rawscope_render::{gpu_timeline_density, visual_field_density};

const DENSITY_GRID_WIDTH: u32 = 512;
const DENSITY_GRID_HEIGHT: u32 = 512;
const BENCHMARK_SEED: u64 = 42;
const GPU_BENCH_ENV: &str = "RAWSCOPE_ENABLE_GPU_BENCH";
const GPU_ROW_COUNTS: [usize; 2] = [200_000, 1_000_000];

fn gpu_density_benchmarks(c: &mut Criterion) {
    let gpu_bench_enabled = env::var_os(GPU_BENCH_ENV).is_some();
    if !gpu_bench_enabled {
        eprintln!(
            "skipping GPU density benchmarks; set {GPU_BENCH_ENV}=1 to opt in on a known compatible machine"
        );
        return;
    }

    let context = match block_on(ComputeContext::new()) {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping GPU density benchmarks; failed to initialize WGPU compute context: {error}");
            return;
        }
    };

    let mut scatter_group = c.benchmark_group("gpu_scatter_density");
    scatter_group.sample_size(10);
    scatter_group.warm_up_time(Duration::from_millis(500));
    scatter_group.measurement_time(Duration::from_secs(2));
    for row_count in GPU_ROW_COUNTS {
        let dataset =
            generate_synthetic_points(SyntheticPointConfig::new(BENCHMARK_SEED, row_count));
        scatter_group.throughput(Throughput::Elements(row_count as u64));
        scatter_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    let grid = visual_field_density(
                        &context,
                        black_box(&dataset.points),
                        dataset.x_range,
                        dataset.y_range,
                        DENSITY_GRID_WIDTH,
                        DENSITY_GRID_HEIGHT,
                    )
                    .expect("GPU scatter benchmark should produce density counts");
                    black_box(grid.total_count())
                });
            },
        );
    }
    scatter_group.finish();

    let mut timeline_group = c.benchmark_group("gpu_timeline_density");
    timeline_group.sample_size(10);
    timeline_group.warm_up_time(Duration::from_millis(500));
    timeline_group.measurement_time(Duration::from_secs(2));
    for row_count in GPU_ROW_COUNTS {
        let dataset =
            generate_synthetic_events(SyntheticEventConfig::new(BENCHMARK_SEED, row_count));
        timeline_group.throughput(Throughput::Elements(row_count as u64));
        timeline_group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    let grid = gpu_timeline_density(
                        &context,
                        black_box(&dataset.events),
                        dataset.time_range,
                        dataset.lane_count,
                        DENSITY_GRID_WIDTH,
                        DENSITY_GRID_HEIGHT,
                    )
                    .expect("GPU timeline benchmark should produce density counts");
                    black_box(grid.total_count())
                });
            },
        );
    }
    timeline_group.finish();
}

criterion_group!(benches, gpu_density_benchmarks);
criterion_main!(benches);
