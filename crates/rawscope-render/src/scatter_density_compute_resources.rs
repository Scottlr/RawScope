//! Buffer, layout, and bind-group construction for resident scatter compute.

use rawscope_data::ScatterPointRecord;

use crate::gpu_scatter_density_pack::{pack_points, ScatterParams};

pub(super) fn point_buffer(
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

pub(super) fn filter_mask_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    point_count: usize,
) -> wgpu::Buffer {
    let included = vec![1_u32; point_count];
    let included_bytes = bytemuck::cast_slice(&included);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Resident Scatter Filter Mask"),
        size: included_bytes.len().max(4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, included_bytes);
    buffer
}

pub(super) fn params_buffers(device: &wgpu::Device, count: usize) -> Vec<wgpu::Buffer> {
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

pub(super) fn grid_buffers(
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

pub(super) fn compute_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Resident Scatter Compute Layout"),
        entries: &(0..5)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 1 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: binding == 0 || binding == 4,
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

pub(super) fn bind_groups(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    points: &wgpu::Buffer,
    filter_mask: &wgpu::Buffer,
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
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: filter_mask.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect()
    })
}

fn count_size_bytes(width: u32, height: u32) -> u64 {
    u64::from(width) * u64::from(height) * 4
}
