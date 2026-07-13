use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use bytemuck::{Pod, Zeroable};
use rawscope_analysis::{
    cohort::{CohortBuilder, CohortGenerationCounter, CohortPolicy},
    visual_field::CategoryLayerPlan,
};
use rawscope_core::{ColumnId, F32Range, RowId};
use rawscope_data::{
    build_visual_field_catalog, CategoryCodeLayout, ColumnChunk, DatasetChunk,
    DatasetGenerationCounter, DatasetIdentity, DatasetMemoryBudget, DatasetSchema,
    DatasetStoreBuilder, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable,
    NormalizedValue, SourceValue, StoreColumnKind, StoredCell, VisualFieldCatalogConfig,
};
use rawscope_gpu::{ComputeContext, DeviceGeneration};
use rawscope_render::{
    clear_pending_layer_counts, composition_bind_group, composition_bind_group_layout,
    composition_compute_pipeline, composition_reference, CategoryChannelGpuResources,
    CategoryChannelIdentity, CategoryLayerPlanGpuResources, CompositionParams, VisualFieldPoint,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Point {
    x: f32,
    y: f32,
}

impl VisualFieldPoint for Point {
    fn x(&self) -> f32 {
        self.x
    }

    fn y(&self) -> f32 {
        self.y
    }
}

fn upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    size: u64,
    usage: wgpu::BufferUsages,
    bytes: &[u8],
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage,
        mapped_at_creation: false,
    });
    if !bytes.is_empty() {
        queue.write_buffer(&buffer, 0, bytes);
    }
    buffer
}

fn fixture() -> (Vec<Point>, Vec<u32>, CategoryCodeLayout, CategoryLayerPlan) {
    let values = ["a", "b", "a", "b"];
    let schema = DatasetSchema::try_new([("category", StoreColumnKind::Utf8)]).unwrap();
    let cells = values
        .iter()
        .map(|value| {
            StoredCell::value(
                SourceValue::Utf8((*value).into()),
                NormalizedValue::Text((*value).into()),
            )
        })
        .collect();
    let mut dataset_generations = DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(12, values.len()),
        schema,
        DatasetMemoryBudget::new(1_000_000),
        &mut dataset_generations,
    );
    builder
        .append_chunk(DatasetChunk::new(
            RowId(0),
            vec![ColumnChunk::new(ColumnId::new(0), cells)],
        ))
        .unwrap();
    let store = builder.finish();
    let source = LoadedSourceTable {
        columns: vec![LoadedColumnSchema {
            name: "category".into(),
            kind: LoadedColumnKind::String,
        }],
        rows: values
            .iter()
            .enumerate()
            .map(|(index, value)| LoadedSourceRow {
                row_id: RowId(index as u64),
                values: vec![(*value).to_string()],
            })
            .collect(),
    };
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    let mut cohort_generations = CohortGenerationCounter::default();
    let cohort = CohortBuilder::new(CohortPolicy::default())
        .evaluate(
            &source,
            &catalog,
            store.generation(),
            &mut cohort_generations,
        )
        .unwrap();
    let index = store
        .category_index(ColumnId::new(0), std::num::NonZeroUsize::new(4).unwrap())
        .unwrap();
    let plan = CategoryLayerPlan::build(&index, &cohort, &[]).unwrap();
    (
        vec![
            Point { x: 0.1, y: 0.1 },
            Point { x: 0.9, y: 0.1 },
            Point { x: 0.1, y: 0.9 },
            Point { x: 0.9, y: 0.9 },
        ],
        index.row_codes().to_vec(),
        index.code_layout(),
        plan,
    )
}

#[test]
#[ignore = "requires a local WGPU adapter"]
fn gpu_category_composition_matches_cpu_layer_reference() {
    pollster::block_on(async {
        let context = ComputeContext::new().await.unwrap();
        let device = context.device();
        let queue = context.queue();
        let (points, category_codes, code_layout, plan) = fixture();
        let dataset_generation = plan.dataset_generation();
        let channel = Arc::new(
            CategoryChannelGpuResources::new(
                device,
                queue,
                CategoryChannelIdentity::new(
                    dataset_generation,
                    DeviceGeneration(1),
                    ColumnId::new(0),
                ),
                &category_codes,
                code_layout,
            )
            .unwrap(),
        );
        let layer_plan = Arc::new(
            CategoryLayerPlanGpuResources::new(device, queue, &plan, DeviceGeneration(1)).unwrap(),
        );
        let points_buffer = upload_buffer(
            device,
            queue,
            "composition parity points",
            (points.len() * std::mem::size_of::<Point>()) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(&points),
        );
        let params = CompositionParams::new(0.0, 1.0, 0.0, 1.0, 2, 2, 0, points.len() as u32);
        let params_buffer = upload_buffer(
            device,
            queue,
            "composition parity params",
            std::mem::size_of::<CompositionParams>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            bytemuck::bytes_of(&params),
        );
        let mask = vec![1_u32; points.len()];
        let mask_buffer = upload_buffer(
            device,
            queue,
            "composition parity mask",
            (mask.len() * std::mem::size_of::<u32>()) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(&mask),
        );
        let cpu = composition_reference(
            &points,
            &category_codes,
            code_layout,
            layer_plan.value_to_layer(),
            layer_plan.special_layer_params(),
            Some(&mask),
            F32Range::new(0.0, 1.0),
            F32Range::new(0.0, 1.0),
            2,
            2,
        )
        .unwrap();
        let total_buffer = upload_buffer(
            device,
            queue,
            "composition parity total",
            (cpu.total_counts().len() * std::mem::size_of::<u32>()) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(&cpu.total_counts()),
        );
        let layer_bytes = (cpu.counts().len() * std::mem::size_of::<u32>()) as u64;
        let pending_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composition parity pending"),
            size: layer_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let bind_group_layout = composition_bind_group_layout(device);
        let bind_group = composition_bind_group(
            device,
            &bind_group_layout,
            &points_buffer,
            &params_buffer,
            &total_buffer,
            &mask_buffer,
            &channel,
            &layer_plan,
            &pending_buffer,
        );
        let pipeline = composition_compute_pipeline(device, &bind_group_layout);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("composition parity encoder"),
        });
        clear_pending_layer_counts(&mut encoder, &pending_buffer, layer_bytes);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("composition parity pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composition parity readback"),
            size: layer_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&pending_buffer, 0, &readback, 0, layer_bytes);
        queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(2)),
            })
            .unwrap();
        let completion = Arc::new(Mutex::new(None));
        let callback_completion = Arc::clone(&completion);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
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
            .expect("composition readback callback")
            .unwrap();
        let mapped = readback.slice(..).get_mapped_range();
        let gpu_counts = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
        assert_eq!(gpu_counts, cpu.counts());
        assert_eq!(
            sum_layers(&gpu_counts, cpu.layer_count(), 4),
            cpu.total_counts()
        );
        drop(mapped);
        readback.unmap();
    });
}

fn sum_layers(layer_counts: &[u32], layer_count: u8, bin_count: usize) -> Vec<u32> {
    let mut totals = vec![0_u32; bin_count];
    for layer in layer_counts
        .chunks_exact(bin_count)
        .take(usize::from(layer_count))
    {
        for (total, count) in totals.iter_mut().zip(layer) {
            *total += *count;
        }
    }
    totals
}
