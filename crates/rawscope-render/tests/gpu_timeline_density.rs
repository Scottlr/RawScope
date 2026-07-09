use rawscope_core::{DensityGrid, RowId, U64Range};
use rawscope_data::{
    generate_synthetic_events, SyntheticEventConfig, SyntheticEventType, TimelineEventKind,
    TimelineEventRecord,
};
use rawscope_gpu::ComputeContext;
use rawscope_render::{gpu_timeline_density, timeline_density};

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_timeline_density_matches_cpu_reference_for_synthetic_events() {
    pollster::block_on(async {
        let context = ComputeContext::new()
            .await
            .expect("WGPU compute context should initialize");
        let dataset = generate_synthetic_events(SyntheticEventConfig::new(42, 512));
        let width = 40;
        let height = dataset.lane_count;

        let cpu_grid = timeline_density(
            &dataset.events,
            dataset.time_range,
            dataset.lane_count,
            width,
            height,
        );
        let gpu_grid = gpu_timeline_density(
            &context,
            &dataset.events,
            dataset.time_range,
            dataset.lane_count,
            width,
            height,
        )
        .await
        .expect("GPU timeline-density should complete");

        assert_eq!(gpu_grid.width(), width);
        assert_eq!(gpu_grid.height(), height);
        assert_eq!(gpu_grid.counts(), cpu_counts(&cpu_grid));
        assert_eq!(gpu_grid.total_count(), cpu_grid.total_row_count());
    });
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_timeline_density_matches_cpu_reference_for_edges_and_out_of_range_events() {
    pollster::block_on(async {
        let context = ComputeContext::new()
            .await
            .expect("WGPU compute context should initialize");
        let time_range = U64Range::new(100, 200);
        let lane_count = 4;
        let width = 10;
        let height = 4;
        let events = vec![
            event(0, 100, 0),
            event(1, 200, 3),
            event(2, 150, 2),
            event(3, 99, 0),
            event(4, 201, 0),
            event(5, 150, 4),
        ];

        let cpu_grid = timeline_density(&events, time_range, lane_count, width, height);
        let gpu_grid =
            gpu_timeline_density(&context, &events, time_range, lane_count, width, height)
                .await
                .expect("GPU timeline-density should complete");

        assert_eq!(gpu_grid.counts(), cpu_counts(&cpu_grid));
        assert_eq!(gpu_grid.count(0, 0), 1);
        assert_eq!(gpu_grid.count(9, 3), 1);
        assert_eq!(gpu_grid.count(5, 2), 1);
        assert_eq!(gpu_grid.total_count(), 3);
    });
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_timeline_density_preserves_injected_pattern_bins() {
    pollster::block_on(async {
        let context = ComputeContext::new()
            .await
            .expect("WGPU compute context should initialize");
        let dataset = generate_synthetic_events(SyntheticEventConfig::new(33, 400));
        let width = 50;
        let height = dataset.lane_count;

        let cpu_grid = timeline_density(
            &dataset.events,
            dataset.time_range,
            dataset.lane_count,
            width,
            height,
        );
        let gpu_grid = gpu_timeline_density(
            &context,
            &dataset.events,
            dataset.time_range,
            dataset.lane_count,
            width,
            height,
        )
        .await
        .expect("GPU timeline-density should complete");

        assert_eq!(gpu_grid.counts(), cpu_counts(&cpu_grid));

        let spike_window_bins = 23..=25;
        let before_spike_window_bins = 20..=22;
        let gap_window_bins = 11..=13;
        let stale_lane_y_bin = dataset.lane_count - 1;
        let late_time_x_bin = 40;

        let spike_total = sum_gpu_timeline_bins(&gpu_grid, spike_window_bins, dataset.lane_count);
        let before_spike_total =
            sum_gpu_timeline_bins(&gpu_grid, before_spike_window_bins, dataset.lane_count);
        let gap_total = sum_gpu_timeline_bins(&gpu_grid, gap_window_bins, dataset.lane_count);
        let stale_lane_late_total = gpu_grid.count(late_time_x_bin, stale_lane_y_bin);

        assert!(spike_total > before_spike_total);
        assert_eq!(gap_total, 0);
        assert_eq!(stale_lane_late_total, 0);
    });
}

fn cpu_counts(grid: &DensityGrid) -> Vec<u32> {
    grid.bins().iter().map(|bin| bin.row_count).collect()
}

fn sum_gpu_timeline_bins(
    grid: &rawscope_render::GpuTimelineDensityGrid,
    x_bins: std::ops::RangeInclusive<u32>,
    lane_count: u32,
) -> u32 {
    x_bins
        .flat_map(|x_bin| {
            let lane_bins = 0..lane_count;
            lane_bins.map(move |lane_bin| grid.count(x_bin, lane_bin))
        })
        .sum()
}

fn event(row_id: u64, timestamp: u64, lane: u32) -> TimelineEventRecord {
    TimelineEventRecord {
        row_id: RowId(row_id),
        timestamp,
        lane,
        value: 1.0,
        kind: TimelineEventKind::Synthetic(SyntheticEventType::Background),
    }
}
