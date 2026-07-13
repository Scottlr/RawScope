use std::{sync::Arc, time::Duration};

use rawscope_analysis::visual_field::{
    derive_density_ridges, RidgeConfig, VisualFieldMapping, VisualFieldProjection,
};
use rawscope_core::{ColumnId, DensityCountGrid, GridSize};
use rawscope_data::{DatasetGenerationCounter, DatasetSchema, StoreColumnKind};
use rawscope_gpu::{ComputeContext, DeviceGeneration};
use rawscope_render::{
    ridge_compute_bind_group, ridge_compute_bind_group_layout, ridge_compute_pipeline,
    ridge_dispatch, ridge_render_bind_group, ridge_render_bind_group_layout, ridge_render_pipeline,
    RidgeFieldGpuResources, RidgeGpuParams, RidgeRenderParams, RidgeResourcePlan,
    VisualFieldDatasetGpuResources, VisualFieldGeneration, VisualFieldQuality,
    VisualFieldViewGenerationCounter,
};

fn source(view: rawscope_render::VisualFieldViewGeneration) -> VisualFieldGeneration {
    let schema =
        DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)]).unwrap();
    let mapping = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        None,
    )
    .unwrap();
    let mut datasets = DatasetGenerationCounter::default();
    let resources = Arc::new(VisualFieldDatasetGpuResources::new(
        datasets.mint(),
        DeviceGeneration(1),
        mapping,
        0,
        0,
    ));
    let mut cohorts = rawscope_analysis::cohort::CohortGenerationCounter::default();
    VisualFieldGeneration::new(
        resources,
        cohorts.mint(),
        view,
        GridSize::new(9, 9),
        VisualFieldQuality::Exact,
    )
}

fn map_readback(device: &wgpu::Device, buffer: &wgpu::Buffer, bytes: u64) -> Vec<u8> {
    let slice = buffer.slice(..bytes);
    let completion = Arc::new(std::sync::Mutex::new(None));
    let callback_completion = Arc::clone(&completion);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        *callback_completion.lock().unwrap() = Some(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(2)),
        })
        .unwrap();
    completion
        .lock()
        .unwrap()
        .take()
        .expect("ridge readback callback")
        .unwrap();
    let mapped = slice.get_mapped_range().to_vec();
    buffer.unmap();
    mapped
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_ridge_stages_match_cpu_reference_vectors() {
    pollster::block_on(async {
        let context = ComputeContext::new().await.unwrap();
        let device = context.device();
        let queue = context.queue();
        let size = GridSize::new(9, 9);
        let counts = DensityCountGrid::new(
            size,
            (0..81)
                .map(|index| {
                    let x: usize = index % 9;
                    let y: usize = index / 9;
                    u32::from((x.abs_diff(4) + y.abs_diff(4)) <= 2)
                })
                .collect(),
        );
        let config = RidgeConfig::default();
        let cpu = derive_density_ridges(&counts, config).unwrap();
        let mut views = VisualFieldViewGenerationCounter::default();
        let source = Arc::new(source(views.mint()));
        let resources =
            RidgeFieldGpuResources::new(device, &source, DeviceGeneration(1), config).unwrap();
        let plan = RidgeResourcePlan::for_grid(size, &device.limits()).unwrap();
        let count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge parity counts"),
            size: plan.grid.bin_count() as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&count_buffer, 0, bytemuck::cast_slice(counts.counts()));
        let params = RidgeGpuParams::for_config(plan, config);
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge parity params"),
            size: std::mem::size_of::<RidgeGpuParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
        let layout = ridge_compute_bind_group_layout(device);
        let bind_group =
            ridge_compute_bind_group(device, &layout, &count_buffer, &params_buffer, &resources);
        let pipeline = ridge_compute_pipeline(device, layout);
        let render_layout = ridge_render_bind_group_layout(device);
        let render_pipeline =
            ridge_render_pipeline(device, render_layout, wgpu::TextureFormat::Bgra8UnormSrgb);
        let render_params = RidgeRenderParams {
            grid_width: size.width(),
            grid_height: size.height(),
            _padding: [0; 2],
            plot_scale_x: 1.0,
            plot_scale_y: 1.0,
            mark_length: 0.02,
            opacity: 0.75,
        };
        let render_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge parity render params"),
            size: std::mem::size_of::<RidgeRenderParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&render_params_buffer, 0, bytemuck::bytes_of(&render_params));
        let _render_bind_group = ridge_render_bind_group(
            device,
            &render_pipeline.layout,
            &resources,
            &render_params_buffer,
        );
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge parity readback"),
            size: plan.ridge_cells_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ridge parity encoder"),
        });
        ridge_dispatch(&mut encoder, &pipeline, &bind_group, &resources, plan);
        encoder.copy_buffer_to_buffer(&resources.cells, 0, &readback, 0, plan.ridge_cells_bytes);
        queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(2)),
            })
            .unwrap();
        let bytes = map_readback(device, &readback, plan.ridge_cells_bytes);
        let gpu = bytemuck::cast_slice::<u8, rawscope_render::GpuRidgeCell>(&bytes);
        assert_eq!(gpu.len(), cpu.cells.len());
        for (gpu, cpu) in gpu.iter().zip(cpu.cells.iter()) {
            assert!((gpu.strength - cpu.strength).abs() < 0.02);
            if cpu.strength > 0.0 {
                assert!((gpu.tangent_x - cpu.tangent_x).abs() < 0.03);
                assert!((gpu.tangent_y - cpu.tangent_y).abs() < 0.03);
            }
        }
    });
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn recovery_rebuilds_ridge_resources_from_source_generation() {
    pollster::block_on(async {
        let context = ComputeContext::new().await.unwrap();
        let device = context.device();
        let mut views = VisualFieldViewGenerationCounter::default();
        let first_source = source(views.mint());
        let second_source = source(views.mint());
        let first = RidgeFieldGpuResources::new(
            device,
            &first_source,
            DeviceGeneration(1),
            RidgeConfig::default(),
        )
        .unwrap();
        let rebuilt = RidgeFieldGpuResources::rebuild_from_source_generation(
            device,
            &second_source,
            DeviceGeneration(1),
            RidgeConfig::default(),
        )
        .unwrap();
        assert_ne!(first.generation, rebuilt.generation);
        assert_eq!(rebuilt.generation, second_source.view_generation());
        assert_eq!(rebuilt.device_generation, DeviceGeneration(1));
    });
}
