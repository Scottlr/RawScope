//! GPU scatter-density compute reference for correctness checks.

use std::{
    error::Error,
    fmt,
    sync::mpsc::{self, RecvError},
};

use bytemuck::{Pod, Zeroable};
use rawscope_core::F32Range;
use rawscope_data::SyntheticPointRecord;
use rawscope_gpu::ComputeContext;

const WORKGROUP_SIZE: u32 = 64;
const EMPTY_BUFFER_SIZE_BYTES: u64 = 4;
const SHADER_SOURCE: &str = include_str!("shaders/scatter_density.wgsl");

/// Flattened GPU scatter-density counts for a 2D grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuScatterDensityGrid {
    width: u32,
    height: u32,
    counts: Vec<u32>,
}

impl GpuScatterDensityGrid {
    /// Creates a flattened GPU count grid.
    pub fn new(width: u32, height: u32, counts: Vec<u32>) -> Self {
        assert!(width > 0, "grid width must be positive");
        assert!(height > 0, "grid height must be positive");
        assert_eq!(
            counts.len(),
            (width as usize) * (height as usize),
            "count length must match grid dimensions"
        );

        Self {
            width,
            height,
            counts,
        }
    }

    /// Returns the grid width in bins.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the grid height in bins.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the flattened row-count bins.
    pub fn counts(&self) -> &[u32] {
        &self.counts
    }

    /// Returns one bin count by x/y coordinate.
    pub fn count(&self, x: u32, y: u32) -> u32 {
        assert!(x < self.width, "x bin out of range");
        assert!(y < self.height, "y bin out of range");

        let bin_index = (y as usize) * (self.width as usize) + (x as usize);
        self.counts[bin_index]
    }

    /// Returns the total number of binned rows.
    pub fn total_count(&self) -> u64 {
        self.counts.iter().map(|count| *count as u64).sum()
    }
}

/// Errors returned by GPU scatter-density compute and readback.
#[derive(Debug)]
pub enum GpuScatterDensityError {
    PointCountTooLarge { point_count: usize },
    BufferMap(wgpu::BufferAsyncError),
    BufferMapCallbackDropped(RecvError),
    DevicePoll(wgpu::PollError),
}

impl fmt::Display for GpuScatterDensityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointCountTooLarge { point_count } => {
                write!(f, "point count {point_count} exceeds u32::MAX")
            }
            Self::BufferMap(err) => write!(f, "failed to map GPU scatter-density readback: {err}"),
            Self::BufferMapCallbackDropped(err) => {
                write!(
                    f,
                    "GPU scatter-density readback callback did not run: {err}"
                )
            }
            Self::DevicePoll(err) => write!(f, "failed while polling GPU device: {err}"),
        }
    }
}

impl Error for GpuScatterDensityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PointCountTooLarge { .. } => None,
            Self::BufferMap(err) => Some(err),
            Self::BufferMapCallbackDropped(err) => Some(err),
            Self::DevicePoll(err) => Some(err),
        }
    }
}

/// Bins point records into a 2D density grid using a WGPU compute shader.
///
/// This correctness path returns counts only. Row-id drilldown remains owned by the CPU
/// reference until a later GPU milestone introduces an explicit row-evidence design.
pub async fn gpu_scatter_density(
    context: &ComputeContext,
    points: &[SyntheticPointRecord],
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> Result<GpuScatterDensityGrid, GpuScatterDensityError> {
    let point_count =
        u32::try_from(points.len()).map_err(|_| GpuScatterDensityError::PointCountTooLarge {
            point_count: points.len(),
        })?;

    let grid_bin_count = (width as usize) * (height as usize);
    if point_count == 0 {
        return Ok(GpuScatterDensityGrid::new(
            width,
            height,
            vec![0; grid_bin_count],
        ));
    }

    let device = context.device();
    let queue = context.queue();
    let packed_points = pack_points(points);
    let params = ScatterParams::new(x_range, y_range, width, height, point_count);

    let point_buffer = create_storage_upload_buffer(
        device,
        queue,
        "RawScope Scatter Point Buffer",
        bytemuck::cast_slice(&packed_points),
    );
    let params_buffer = create_uniform_upload_buffer(
        device,
        queue,
        "RawScope Scatter Params Buffer",
        bytemuck::bytes_of(&params),
    );

    let output_size_bytes = (grid_bin_count * std::mem::size_of::<u32>()) as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Scatter Density Output Buffer"),
        size: output_size_bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    clear_output_buffer(queue, &output_buffer, output_size_bytes);

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Scatter Density Readback Buffer"),
        size: output_size_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RawScope Scatter Density Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });
    let bind_group_layout = create_bind_group_layout(device);
    let bind_group = create_bind_group(
        device,
        &bind_group_layout,
        &point_buffer,
        &params_buffer,
        &output_buffer,
    );
    let pipeline = create_compute_pipeline(device, &bind_group_layout, &shader);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("RawScope Scatter Density Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RawScope Scatter Density Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = point_count.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &readback_buffer, 0, output_size_bytes);
    queue.submit(Some(encoder.finish()));

    let counts = readback_counts(device, &readback_buffer, grid_bin_count)?;
    Ok(GpuScatterDensityGrid::new(width, height, counts))
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuPoint {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ScatterParams {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    grid_width: u32,
    grid_height: u32,
    point_count: u32,
    _padding: u32,
}

impl ScatterParams {
    fn new(
        x_range: F32Range,
        y_range: F32Range,
        grid_width: u32,
        grid_height: u32,
        point_count: u32,
    ) -> Self {
        Self {
            x_min: x_range.min,
            x_max: x_range.max,
            y_min: y_range.min,
            y_max: y_range.max,
            grid_width,
            grid_height,
            point_count,
            _padding: 0,
        }
    }
}

fn pack_points(points: &[SyntheticPointRecord]) -> Vec<GpuPoint> {
    points
        .iter()
        .map(|point| GpuPoint {
            x: point.x,
            y: point.y,
        })
        .collect()
}

fn create_storage_upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    bytes: &[u8],
) -> wgpu::Buffer {
    let buffer_size_bytes = bytes.len() as u64;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: buffer_size_bytes.max(EMPTY_BUFFER_SIZE_BYTES),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytes);
    buffer
}

fn create_uniform_upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    bytes: &[u8],
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytes);
    buffer
}

fn clear_output_buffer(queue: &wgpu::Queue, buffer: &wgpu::Buffer, output_size_bytes: u64) {
    let zeroed_output = vec![0_u8; output_size_bytes as usize];
    queue.write_buffer(buffer, 0, &zeroed_output);
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Scatter Density Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    point_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    output_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Scatter Density Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: point_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    })
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::ComputePipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Scatter Density Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("RawScope Scatter Density Pipeline"),
        layout: Some(&pipeline_layout),
        module: shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn readback_counts(
    device: &wgpu::Device,
    readback_buffer: &wgpu::Buffer,
    grid_bin_count: usize,
) -> Result<Vec<u32>, GpuScatterDensityError> {
    let readback_slice = readback_buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    readback_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(GpuScatterDensityError::DevicePoll)?;

    receiver
        .recv()
        .map_err(GpuScatterDensityError::BufferMapCallbackDropped)?
        .map_err(GpuScatterDensityError::BufferMap)?;

    let counts = {
        let mapped = readback_slice.get_mapped_range();
        bytemuck::cast_slice::<u8, u32>(&mapped)[..grid_bin_count].to_vec()
    };
    readback_buffer.unmap();

    Ok(counts)
}
