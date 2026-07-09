//! GPU scatter-density compute reference for correctness checks.

use std::{error::Error, fmt, sync::mpsc::RecvError};

use rawscope_core::{DensityCountGrid, F32Range, GridSize};
use rawscope_data::ScatterPointRecord;
use rawscope_gpu::ComputeContext;

use crate::gpu_density_pipeline::{
    clear_output_buffer, create_density_bind_group, create_density_bind_group_layout,
    create_density_compute_pipeline, create_storage_upload_buffer, create_uniform_upload_buffer,
    readback_counts_from_buffer, GpuDensityReadbackError,
};
use crate::gpu_scatter_density_pack::{pack_points, ScatterParams};

const WORKGROUP_SIZE: u32 = 64;
const MAX_DISPATCH_WORKGROUPS_PER_DIMENSION: u32 = 65_535;
const MAX_POINTS_PER_DISPATCH: u32 = WORKGROUP_SIZE * MAX_DISPATCH_WORKGROUPS_PER_DIMENSION;
const SHADER_SOURCE: &str = include_str!("shaders/scatter_density.wgsl");

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScatterDensityComputeConfig {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
}

/// Flattened GPU scatter-density counts for a 2D grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuScatterDensityGrid {
    counts: DensityCountGrid,
}

impl GpuScatterDensityGrid {
    /// Creates a flattened GPU count grid.
    pub fn new(width: u32, height: u32, counts: Vec<u32>) -> Self {
        Self {
            counts: DensityCountGrid::new(GridSize::new(width, height), counts),
        }
    }

    /// Returns the grid width in bins.
    pub fn width(&self) -> u32 {
        self.counts.width()
    }

    /// Returns the grid height in bins.
    pub fn height(&self) -> u32 {
        self.counts.height()
    }

    /// Returns the flattened row-count bins.
    pub fn counts(&self) -> &[u32] {
        self.counts.counts()
    }

    /// Returns one bin count by x/y coordinate.
    pub fn count(&self, x: u32, y: u32) -> u32 {
        self.counts.count(x, y)
    }

    /// Returns the total number of binned rows.
    pub fn total_count(&self) -> u64 {
        self.counts.total_count()
    }
}

/// Errors returned by GPU scatter-density compute and readback.
#[derive(Debug)]
pub enum GpuScatterDensityError {
    PointCountTooLarge { point_count: usize },
    MissingReadbackCounts,
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
            Self::MissingReadbackCounts => write!(
                f,
                "GPU scatter-density readback counts were requested but not returned"
            ),
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
            Self::PointCountTooLarge { .. } | Self::MissingReadbackCounts => None,
            Self::BufferMap(err) => Some(err),
            Self::BufferMapCallbackDropped(err) => Some(err),
            Self::DevicePoll(err) => Some(err),
        }
    }
}

impl From<GpuDensityReadbackError> for GpuScatterDensityError {
    fn from(err: GpuDensityReadbackError) -> Self {
        match err {
            GpuDensityReadbackError::BufferMap(err) => Self::BufferMap(err),
            GpuDensityReadbackError::BufferMapCallbackDropped(err) => {
                Self::BufferMapCallbackDropped(err)
            }
            GpuDensityReadbackError::DevicePoll(err) => Self::DevicePoll(err),
        }
    }
}

/// Bins point records into a 2D density grid using a WGPU compute shader.
///
/// This correctness path returns counts only. Row-id drilldown remains owned by the CPU
/// reference until a later GPU milestone introduces an explicit row-evidence design.
pub async fn gpu_scatter_density(
    context: &ComputeContext,
    points: &[ScatterPointRecord],
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> Result<GpuScatterDensityGrid, GpuScatterDensityError> {
    gpu_scatter_density_on_device(
        context.device(),
        context.queue(),
        points,
        x_range,
        y_range,
        width,
        height,
    )
    .await
}

/// Bins point records using an existing WGPU device and queue.
///
/// This is used by the workbench visual path to avoid creating a second headless
/// compute device alongside the window rendering device.
pub async fn gpu_scatter_density_on_device(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    points: &[ScatterPointRecord],
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

    let compute_config = ScatterDensityComputeConfig {
        x_range,
        y_range,
        grid_width: width,
        grid_height: height,
    };
    let compute_output = dispatch_scatter_density(device, queue, points, compute_config, true)?;
    let counts = compute_output
        .counts
        .ok_or(GpuScatterDensityError::MissingReadbackCounts)?;
    Ok(GpuScatterDensityGrid::new(width, height, counts))
}

pub(crate) struct ScatterDensityComputeOutput {
    pub count_buffer: wgpu::Buffer,
    pub counts: Option<Vec<u32>>,
}

pub(crate) fn dispatch_scatter_density(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    points: &[ScatterPointRecord],
    config: ScatterDensityComputeConfig,
    readback_counts: bool,
) -> Result<ScatterDensityComputeOutput, GpuScatterDensityError> {
    let point_count =
        u32::try_from(points.len()).map_err(|_| GpuScatterDensityError::PointCountTooLarge {
            point_count: points.len(),
        })?;

    let grid_bin_count = (config.grid_width as usize) * (config.grid_height as usize);
    let packed_points = pack_points(points);
    let point_buffer = create_storage_upload_buffer(
        device,
        queue,
        "RawScope Scatter Point Buffer",
        bytemuck::cast_slice(&packed_points),
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
    let bind_group_layout =
        create_density_bind_group_layout(device, "RawScope Scatter Density Bind Group Layout");
    let pipeline = create_density_compute_pipeline(
        device,
        "RawScope Scatter Density Pipeline Layout",
        "RawScope Scatter Density Pipeline",
        &bind_group_layout,
        &shader,
    );
    let dispatch_chunks = scatter_dispatch_chunks(point_count);
    let dispatch_bind_groups = create_dispatch_bind_groups(
        device,
        queue,
        &bind_group_layout,
        &point_buffer,
        &output_buffer,
        config,
        &dispatch_chunks,
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("RawScope Scatter Density Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RawScope Scatter Density Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        for dispatch_bind_group in &dispatch_bind_groups {
            let chunk_workgroup_count = dispatch_bind_group
                .chunk
                .point_count
                .div_ceil(WORKGROUP_SIZE);
            compute_pass.set_bind_group(0, &dispatch_bind_group.bind_group, &[]);
            compute_pass.dispatch_workgroups(chunk_workgroup_count, 1, 1);
        }
    }

    if readback_counts {
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &readback_buffer, 0, output_size_bytes);
    }
    queue.submit(Some(encoder.finish()));

    let counts = if readback_counts {
        Some(readback_counts_from_buffer(
            device,
            &readback_buffer,
            grid_bin_count,
        )?)
    } else {
        None
    };

    Ok(ScatterDensityComputeOutput {
        count_buffer: output_buffer,
        counts,
    })
}

struct ScatterDispatchChunk {
    point_start: u32,
    point_count: u32,
}

struct ScatterDispatchBindGroup {
    chunk: ScatterDispatchChunk,
    _params_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

fn scatter_dispatch_chunks(point_count: u32) -> Vec<ScatterDispatchChunk> {
    let mut chunks = Vec::new();
    let mut point_start = 0;

    while point_start < point_count {
        let remaining_point_count = point_count - point_start;
        let chunk_point_count = remaining_point_count.min(MAX_POINTS_PER_DISPATCH);
        chunks.push(ScatterDispatchChunk {
            point_start,
            point_count: chunk_point_count,
        });
        point_start += chunk_point_count;
    }

    chunks
}

fn create_dispatch_bind_groups(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    point_buffer: &wgpu::Buffer,
    output_buffer: &wgpu::Buffer,
    config: ScatterDensityComputeConfig,
    dispatch_chunks: &[ScatterDispatchChunk],
) -> Vec<ScatterDispatchBindGroup> {
    dispatch_chunks
        .iter()
        .map(|chunk| {
            let params = ScatterParams::new(
                config.x_range,
                config.y_range,
                config.grid_width,
                config.grid_height,
                chunk.point_start,
                chunk.point_count,
            );
            let params_buffer = create_uniform_upload_buffer(
                device,
                queue,
                "RawScope Scatter Params Buffer",
                bytemuck::bytes_of(&params),
            );
            let bind_group = create_density_bind_group(
                device,
                "RawScope Scatter Density Bind Group",
                bind_group_layout,
                point_buffer,
                &params_buffer,
                output_buffer,
            );

            ScatterDispatchBindGroup {
                chunk: ScatterDispatchChunk {
                    point_start: chunk.point_start,
                    point_count: chunk.point_count,
                },
                _params_buffer: params_buffer,
                bind_group,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{scatter_dispatch_chunks, MAX_POINTS_PER_DISPATCH};

    #[test]
    fn dispatch_chunks_keep_each_dispatch_within_wgpu_limit() {
        let point_count = MAX_POINTS_PER_DISPATCH + 10;

        let chunks = scatter_dispatch_chunks(point_count);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].point_start, 0);
        assert_eq!(chunks[0].point_count, MAX_POINTS_PER_DISPATCH);
        assert_eq!(chunks[1].point_start, MAX_POINTS_PER_DISPATCH);
        assert_eq!(chunks[1].point_count, 10);
    }
}
