//! Production-path CPU benchmark scenarios for generic visual-field lifecycles.
//!
//! Criterion owns the timing distribution (including p50/p95 reporting). The
//! scenarios exercise the same analysis/render owners used by the workbench:
//! constant-size reprojection, exact count settlement, shared contour/marginal
//! derivation, ridge presentation, semantic zoom, resolution policy, and
//! checked resource accounting. GPU adapter details are emitted as unknown
//! metadata here unless the opt-in GPU benchmark supplies them.

use std::{hint::black_box, sync::Arc, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rawscope_analysis::visual_field::{
    derive_density_ridges, semantic_zoom_frame, DensityRidgeField, RidgeConfig, RidgeScale,
    SettledDensityContext,
};
use rawscope_core::{DensityCountGrid, F32Range, GridSize};
use rawscope_data::{generate_synthetic_points, SyntheticPointConfig};
use rawscope_render::{
    estimate_visual_field_resources, scatter_density, VisualFieldQuality, VisualFieldReprojection,
    VisualFieldResourceOptions, VisualFieldViewport,
};

const BENCHMARK_SEED: u64 = 42;
const ROW_COUNTS: [usize; 2] = [2_048, 16_384];
const GRID_WIDTH: u32 = 128;
const GRID_HEIGHT: u32 = 96;

fn interaction_coordinator_benchmarks(c: &mut Criterion) {
    let mut pointer_group = c.benchmark_group("visual_field_pointer_submission");
    pointer_group.sample_size(20);
    pointer_group.warm_up_time(Duration::from_millis(300));
    pointer_group.measurement_time(Duration::from_secs(2));
    let reprojection = reprojection_fixture();
    pointer_group.throughput(Throughput::Elements(1));
    pointer_group.bench_function("pan_zoom_hover_constant_size", |bencher| {
        bencher.iter(|| {
            let mut checksum = 0.0_f32;
            for (u, v) in [(0.05, 0.15), (0.25, 0.50), (0.75, 0.80), (0.95, 0.10)] {
                if let Some((x, y)) = reprojection.source_uv(black_box(u), black_box(v)) {
                    checksum += x + y;
                }
            }
            black_box(checksum)
        });
    });
    pointer_group.finish();

    let mut settle_group = c.benchmark_group("visual_field_settle_and_derived");
    settle_group.sample_size(10);
    settle_group.warm_up_time(Duration::from_millis(500));
    settle_group.measurement_time(Duration::from_secs(2));
    for row_count in ROW_COUNTS {
        let dataset =
            generate_synthetic_points(SyntheticPointConfig::new(BENCHMARK_SEED, row_count));
        settle_group.throughput(Throughput::Elements(row_count as u64));
        settle_group.bench_with_input(
            BenchmarkId::new("exact_density_mass_marginals_ridges", row_count),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    let density = scatter_density(
                        black_box(&dataset.points),
                        dataset.x_range,
                        dataset.y_range,
                        GRID_WIDTH,
                        GRID_HEIGHT,
                    );
                    let counts = DensityCountGrid::new(
                        GridSize::new(GRID_WIDTH, GRID_HEIGHT),
                        density.bins().iter().map(|bin| bin.row_count).collect(),
                    );
                    let settled = SettledDensityContext::try_new_with_default_fractions(
                        1_u64,
                        Arc::new(counts),
                    )
                    .expect("settled density context should be internally consistent");
                    let ridge_fields = [RidgeScale::Fine, RidgeScale::Medium, RidgeScale::Coarse]
                        .into_iter()
                        .map(|scale| {
                            derive_density_ridges(
                                &settled.counts,
                                RidgeConfig {
                                    scale,
                                    ..RidgeConfig::default()
                                },
                            )
                        })
                        .collect::<Result<Vec<DensityRidgeField>, _>>()
                        .expect("all reviewed ridge scales should derive");
                    let resources = estimate_visual_field_resources(
                        GridSize::new(GRID_WIDTH, GRID_HEIGHT),
                        VisualFieldResourceOptions {
                            ridge_fields: ridge_fields.len() as u8,
                            ..VisualFieldResourceOptions::default()
                        },
                    )
                    .expect("resource estimate should remain checked");
                    black_box((
                        settled.counts.total_count(),
                        settled.contours.levels.len(),
                        resources.total_bytes(),
                        semantic_zoom_frame(0.05, Default::default()),
                    ))
                });
            },
        );
    }
    settle_group.finish();
}

fn reprojection_fixture() -> VisualFieldReprojection {
    let viewport = VisualFieldViewport {
        x_range: F32Range::new(-1.0, 1.0),
        y_range: F32Range::new(-1.0, 1.0),
        grid_width: GRID_WIDTH,
        grid_height: GRID_HEIGHT,
        viewport_revision: 1,
        quality: VisualFieldQuality::Exact,
    };
    let schema = rawscope_data::DatasetSchema::try_new([
        ("horizontal", rawscope_data::StoreColumnKind::F64),
        ("vertical", rawscope_data::StoreColumnKind::F64),
    ])
    .expect("benchmark schema is valid");
    let mapping = rawscope_analysis::visual_field::VisualFieldMapping::try_new(
        &schema,
        rawscope_analysis::visual_field::VisualFieldProjection::NumericPair {
            x: rawscope_core::ColumnId::new(0),
            y: rawscope_core::ColumnId::new(1),
        },
        None,
    )
    .expect("benchmark mapping is valid");
    VisualFieldReprojection::try_new(
        viewport,
        mapping,
        viewport,
        mapping,
        viewport.x_range,
        viewport.y_range,
    )
    .expect("benchmark reprojection is compatible")
}

criterion_group!(benches, interaction_coordinator_benchmarks);
criterion_main!(benches);
