use rawscope_core::{F32Range, RowId};
use rawscope_data::{
    generate_synthetic_points, ScatterPointKind, ScatterPointRecord, SyntheticPointCategory,
    SyntheticPointConfig,
};
use rawscope_gpu::ComputeContext;
use rawscope_render::{
    gpu_scatter_density, scatter_density, DensityReadbackPolicy, ScatterDensityGpuState,
    ScatterDensityRendererConfig, ScatterDensityUpdate,
};

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_scatter_density_matches_cpu_reference_for_synthetic_points() {
    pollster::block_on(async {
        let context = ComputeContext::new()
            .await
            .expect("WGPU compute context should initialize");
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
#[ignore = "requires a local WGPU adapter"]
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
#[ignore = "requires a local WGPU adapter"]
fn resident_state_reuses_point_capacity_for_view_updates() {
    pollster::block_on(async {
        let context = ComputeContext::new().await.unwrap();
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(7, 256));
        let config = ScatterDensityRendererConfig::new(dataset.x_range, dataset.y_range, 16, 16);
        let mut state = ScatterDensityGpuState::new(
            context.device(),
            context.queue(),
            &dataset.points,
            config,
            1,
        )
        .unwrap();
        let capacity = state.point_capacity();
        state
            .update(
                context.device(),
                context.queue(),
                ScatterDensityUpdate {
                    config,
                    readback: DensityReadbackPolicy::None,
                },
            )
            .unwrap();
        assert_eq!(state.point_capacity(), capacity);
        assert_eq!(state.dataset_revision(), 1);
    });
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn dataset_revision_replaces_resident_point_state() {
    pollster::block_on(async {
        let context = ComputeContext::new().await.unwrap();
        let first = generate_synthetic_points(SyntheticPointConfig::new(7, 128));
        let second = generate_synthetic_points(SyntheticPointConfig::new(8, 320));
        let config = ScatterDensityRendererConfig::new(first.x_range, first.y_range, 16, 16);
        let mut state = ScatterDensityGpuState::new(
            context.device(),
            context.queue(),
            &first.points,
            config,
            1,
        )
        .unwrap();
        state
            .replace_dataset(context.device(), context.queue(), &second.points, 2)
            .unwrap();
        assert_eq!(state.dataset_revision(), 2);
        assert_eq!(state.point_capacity(), second.points.len());
    });
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_max_reduction_matches_cpu_reference() {
    pollster::block_on(async {
        let context = ComputeContext::new().await.unwrap();
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(9, 512));
        let config = ScatterDensityRendererConfig::new(dataset.x_range, dataset.y_range, 24, 20);
        let cpu = scatter_density(&dataset.points, dataset.x_range, dataset.y_range, 24, 20);
        let expected_max = cpu
            .bins()
            .iter()
            .map(|bin| bin.row_count)
            .max()
            .unwrap_or(0);
        let mut state = ScatterDensityGpuState::new(
            context.device(),
            context.queue(),
            &dataset.points,
            config,
            1,
        )
        .unwrap();
        let stats = state
            .update(
                context.device(),
                context.queue(),
                ScatterDensityUpdate {
                    config,
                    readback: DensityReadbackPolicy::MaxOnly,
                },
            )
            .unwrap();
        assert_eq!(stats.max_bin_count, expected_max);
        assert!(stats.max_bin_count_is_current);
    });
}

fn cpu_counts(grid: &rawscope_core::DensityGrid) -> Vec<u32> {
    grid.bins().iter().map(|bin| bin.row_count).collect()
}

fn point(row_id: u64, x: f32, y: f32) -> ScatterPointRecord {
    ScatterPointRecord {
        row_id: RowId(row_id),
        x,
        y,
        kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Background),
    }
}
