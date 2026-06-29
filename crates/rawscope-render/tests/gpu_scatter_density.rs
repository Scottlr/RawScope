use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    generate_synthetic_points, SyntheticPointCategory, SyntheticPointConfig, SyntheticPointRecord,
};
use rawscope_gpu::ComputeContext;
use rawscope_render::{gpu_scatter_density, scatter_density};

const GPU_TEST_COMMAND: &str =
    "cargo test -p rawscope-render --test gpu_scatter_density -- --ignored --nocapture";

#[test]
#[ignore = "requires a local WGPU adapter; run the command in GPU_TEST_COMMAND"]
fn gpu_scatter_density_matches_cpu_reference_for_synthetic_points() {
    pollster::block_on(async {
        let context = ComputeContext::new()
            .await
            .expect("WGPU compute context should initialize");
        let diagnostics = context.diagnostics();
        eprintln!(
            "GPU scatter-density adapter: {} ({}, {})",
            diagnostics.adapter_name, diagnostics.backend, diagnostics.device_type
        );
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(42, 512));
        let width = 32;
        let height = 24;

        let cpu_grid = scatter_density(
            &dataset.points,
            dataset.x_range,
            dataset.y_range,
            width,
            height,
        );
        let gpu_grid = gpu_scatter_density(
            &context,
            &dataset.points,
            dataset.x_range,
            dataset.y_range,
            width,
            height,
        )
        .await
        .expect("GPU scatter-density should complete");

        assert_eq!(gpu_grid.width(), width);
        assert_eq!(gpu_grid.height(), height);
        assert_eq!(gpu_grid.counts(), cpu_counts(&cpu_grid));
        assert_eq!(gpu_grid.total_count(), cpu_grid.total_row_count());
    });
}

#[test]
#[ignore = "requires a local WGPU adapter; run the command in GPU_TEST_COMMAND"]
fn gpu_scatter_density_matches_cpu_reference_for_edges_and_out_of_range_points() {
    pollster::block_on(async {
        let context = ComputeContext::new()
            .await
            .expect("WGPU compute context should initialize");
        let x_range = F32Range::new(0.0, 10.0);
        let y_range = F32Range::new(0.0, 10.0);
        let points = vec![
            point(0, 0.0, 0.0),
            point(1, 10.0, 10.0),
            point(2, 5.0, 5.0),
            point(3, -0.1, 5.0),
            point(4, 5.0, 10.1),
        ];
        let width = 10;
        let height = 10;

        let cpu_grid = scatter_density(&points, x_range, y_range, width, height);
        let gpu_grid = gpu_scatter_density(&context, &points, x_range, y_range, width, height)
            .await
            .expect("GPU scatter-density should complete");

        assert_eq!(gpu_grid.counts(), cpu_counts(&cpu_grid));
        assert_eq!(gpu_grid.count(0, 0), 1);
        assert_eq!(gpu_grid.count(9, 9), 1);
        assert_eq!(gpu_grid.count(5, 5), 1);
        assert_eq!(gpu_grid.total_count(), 3);
    });
}

#[test]
fn gpu_correctness_tests_document_manual_command() {
    assert!(GPU_TEST_COMMAND.contains("--ignored"));
}

fn cpu_counts(grid: &rawscope_core::DensityGrid) -> Vec<u32> {
    grid.bins().iter().map(|bin| bin.row_count).collect()
}

fn point(row_id: u64, x: f32, y: f32) -> SyntheticPointRecord {
    SyntheticPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        category: SyntheticPointCategory::Background,
    }
}
