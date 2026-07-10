//! Dataset-resident scatter density compute resources.

use rawscope_data::ScatterPointRecord;

use crate::{
    gpu_density_pipeline::{readback_counts_from_buffer, GpuDensityReadbackError},
    gpu_scatter_density::GpuScatterDensityError,
    gpu_scatter_density_pack::{pack_points, ScatterParams},
    ScatterDensityRenderStats, ScatterDensityRendererConfig,
};

const WORKGROUP_SIZE: u32 = 64;
const MAX_WORKGROUPS: u32 = 65_535;
const MAX_POINTS_PER_DISPATCH: u32 = WORKGROUP_SIZE * MAX_WORKGROUPS;
const COMPUTE_SHADER: &str = include_str!("shaders/scatter_density.wgsl");

#[path = "scatter_density_gpu_lifecycle.rs"]
mod lifecycle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityReadbackPolicy {
    None,
    MaxOnly,
    FullCounts,
}

impl DensityReadbackPolicy {
    pub fn requests_count_mapping(self) -> bool {
        self == Self::FullCounts
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterDensityUpdate {
    pub config: ScatterDensityRendererConfig,
    pub readback: DensityReadbackPolicy,
}

pub(crate) struct ScatterDensityUpdateOutput {
    pub stats: ScatterDensityRenderStats,
    pub counts: Option<Vec<u32>>,
}

pub struct ScatterDensityGpuState {
    point_buffer: wgpu::Buffer,
    point_count: u32,
    point_capacity: usize,
    count_buffers: [wgpu::Buffer; 2],
    max_count_buffer: wgpu::Buffer,
    params_buffers: Vec<wgpu::Buffer>,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_groups: [Vec<wgpu::BindGroup>; 2],
    full_readback_buffer: wgpu::Buffer,
    max_readback_buffer: wgpu::Buffer,
    active_count_buffer: usize,
    grid_width: u32,
    grid_height: u32,
    count_capacity_bins: u64,
    dataset_revision: u64,
    last_max_bin_count: u32,
    grid_generation: u64,
}

impl ScatterDensityGpuState {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[ScatterPointRecord],
        config: ScatterDensityRendererConfig,
        dataset_revision: u64,
    ) -> Result<Self, GpuScatterDensityError> {
        let point_count = checked_point_count(points)?;
        let point_buffer = point_buffer(device, queue, points);
        let compute_bind_group_layout = compute_layout(device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Resident Scatter Density Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RawScope Resident Scatter Pipeline Layout"),
            bind_group_layouts: &[Some(&compute_bind_group_layout)],
            immediate_size: 0,
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RawScope Resident Scatter Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let (count_buffers, max_count_buffer, full_readback_buffer, max_readback_buffer) =
            grid_buffers(device, config.grid_width, config.grid_height);
        let params_buffers = params_buffers(device, dispatch_chunks(point_count).len());
        let compute_bind_groups = bind_groups(
            device,
            &compute_bind_group_layout,
            &point_buffer,
            &params_buffers,
            &count_buffers,
            &max_count_buffer,
        );
        Ok(Self {
            point_buffer,
            point_count,
            point_capacity: points.len(),
            count_buffers,
            max_count_buffer,
            params_buffers,
            compute_bind_group_layout,
            compute_pipeline,
            compute_bind_groups,
            full_readback_buffer,
            max_readback_buffer,
            active_count_buffer: 0,
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            count_capacity_bins: u64::from(config.grid_width) * u64::from(config.grid_height),
            dataset_revision,
            last_max_bin_count: 0,
            grid_generation: 0,
        })
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update: ScatterDensityUpdate,
    ) -> Result<ScatterDensityRenderStats, GpuScatterDensityError> {
        Ok(self.update_with_output(device, queue, update)?.stats)
    }

    pub(crate) fn update_with_output(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update: ScatterDensityUpdate,
    ) -> Result<ScatterDensityUpdateOutput, GpuScatterDensityError> {
        if (self.grid_width, self.grid_height)
            != (update.config.grid_width, update.config.grid_height)
        {
            self.resize_grid(device, update.config.grid_width, update.config.grid_height);
        }
        let destination = destination_count_buffer(self.active_count_buffer);
        let chunks = dispatch_chunks(self.point_count);
        for (index, chunk) in chunks.iter().enumerate() {
            let params = ScatterParams::new(
                update.config.x_range,
                update.config.y_range,
                self.grid_width,
                self.grid_height,
                chunk.0,
                chunk.1,
            );
            queue.write_buffer(&self.params_buffers[index], 0, bytemuck::bytes_of(&params));
        }
        let count_size = count_size_bytes(self.grid_width, self.grid_height);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RawScope Resident Scatter Update"),
        });
        encoder.clear_buffer(&self.count_buffers[destination], 0, None);
        encoder.clear_buffer(&self.max_count_buffer, 0, None);
        if self.point_count > 0 {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RawScope Resident Scatter Compute"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.compute_pipeline);
            for (index, chunk) in chunks.iter().enumerate() {
                pass.set_bind_group(0, &self.compute_bind_groups[destination][index], &[]);
                pass.dispatch_workgroups(chunk.1.div_ceil(WORKGROUP_SIZE), 1, 1);
            }
        }
        match update.readback {
            DensityReadbackPolicy::None => {}
            DensityReadbackPolicy::MaxOnly => encoder.copy_buffer_to_buffer(
                &self.max_count_buffer,
                0,
                &self.max_readback_buffer,
                0,
                4,
            ),
            DensityReadbackPolicy::FullCounts => {
                encoder.copy_buffer_to_buffer(
                    &self.count_buffers[destination],
                    0,
                    &self.full_readback_buffer,
                    0,
                    count_size,
                );
                encoder.copy_buffer_to_buffer(
                    &self.max_count_buffer,
                    0,
                    &self.max_readback_buffer,
                    0,
                    4,
                );
            }
        }
        queue.submit(Some(encoder.finish()));
        let counts = if update.readback == DensityReadbackPolicy::FullCounts {
            Some(
                readback_counts_from_buffer(
                    device,
                    &self.full_readback_buffer,
                    (self.grid_width * self.grid_height) as usize,
                )
                .map_err(map_readback)?,
            )
        } else {
            None
        };
        let max_is_current = update.readback != DensityReadbackPolicy::None;
        if max_is_current {
            self.last_max_bin_count =
                readback_counts_from_buffer(device, &self.max_readback_buffer, 1)
                    .map_err(map_readback)?[0];
        }
        self.active_count_buffer = destination;
        Ok(ScatterDensityUpdateOutput {
            stats: ScatterDensityRenderStats {
                point_count: self.point_count as usize,
                grid_width: self.grid_width,
                grid_height: self.grid_height,
                max_bin_count: self.last_max_bin_count,
                max_bin_count_is_current: max_is_current,
            },
            counts,
        })
    }
}

fn checked_point_count(points: &[ScatterPointRecord]) -> Result<u32, GpuScatterDensityError> {
    u32::try_from(points.len()).map_err(|_| GpuScatterDensityError::PointCountTooLarge {
        point_count: points.len(),
    })
}
fn destination_count_buffer(active_count_buffer: usize) -> usize {
    1 - active_count_buffer
}
fn dispatch_chunks(count: u32) -> Vec<(u32, u32)> {
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < count {
        let size = (count - start).min(MAX_POINTS_PER_DISPATCH);
        chunks.push((start, size));
        start += size;
    }
    chunks
}
fn count_size_bytes(width: u32, height: u32) -> u64 {
    u64::from(width) * u64::from(height) * 4
}
fn point_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    points: &[ScatterPointRecord],
) -> wgpu::Buffer {
    let packed = pack_points(points);
    let packed_bytes = bytemuck::cast_slice(&packed);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Resident Scatter Points"),
        size: packed_bytes.len().max(4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, packed_bytes);
    buffer
}
fn params_buffers(device: &wgpu::Device, count: usize) -> Vec<wgpu::Buffer> {
    (0..count)
        .map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("RawScope Resident Scatter Params"),
                size: std::mem::size_of::<ScatterParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
        .collect()
}
fn grid_buffers(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> ([wgpu::Buffer; 2], wgpu::Buffer, wgpu::Buffer, wgpu::Buffer) {
    let size = count_size_bytes(width, height);
    let count = || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Resident Scatter Counts"),
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    let max = || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Resident Scatter Maximum"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    let read = |bytes, label| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    (
        [count(), count()],
        max(),
        read(size, "RawScope Scatter Full Readback"),
        read(4, "RawScope Scatter Max Readback"),
    )
}
fn compute_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Resident Scatter Compute Layout"),
        entries: &(0..4)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 1 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: binding == 0,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect::<Vec<_>>(),
    })
}
fn bind_groups(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    points: &wgpu::Buffer,
    params: &[wgpu::Buffer],
    counts: &[wgpu::Buffer; 2],
    max: &wgpu::Buffer,
) -> [Vec<wgpu::BindGroup>; 2] {
    std::array::from_fn(|buffer_index| {
        params
            .iter()
            .map(|params| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("RawScope Resident Scatter Bind Group"),
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: points.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: params.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: counts[buffer_index].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: max.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect()
    })
}
fn map_readback(error: GpuDensityReadbackError) -> GpuScatterDensityError {
    error.into()
}

#[cfg(test)]
#[path = "scatter_density_gpu_state_tests.rs"]
mod tests;
