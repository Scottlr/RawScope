//! GPU timeline-density compute reference for correctness checks.

use std::{error::Error, fmt, sync::mpsc::RecvError};

use rawscope_core::{DensityCountGrid, GridSize, U64Range};
use rawscope_data::TimelineEventRecord;
use rawscope_gpu::ComputeContext;

use crate::gpu_density_pipeline::{
    clear_output_buffer, create_density_bind_group, create_density_bind_group_layout,
    create_density_compute_pipeline, create_storage_upload_buffer, create_uniform_upload_buffer,
    readback_counts_from_buffer, GpuDensityReadbackError,
};
use crate::gpu_timeline_density_pack::{pack_events, timeline_span_u32, TimelineParams};

const WORKGROUP_SIZE: u32 = 64;
const MAX_DISPATCH_WORKGROUPS_PER_DIMENSION: u32 = 65_535;
const MAX_EVENTS_PER_DISPATCH: u32 = WORKGROUP_SIZE * MAX_DISPATCH_WORKGROUPS_PER_DIMENSION;
const SHADER_SOURCE: &str = include_str!("shaders/timeline_density.wgsl");

/// Flattened GPU timeline-density counts for a 2D grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTimelineDensityGrid {
    counts: DensityCountGrid,
}

impl GpuTimelineDensityGrid {
    /// Creates a flattened GPU timeline count grid.
    pub fn new(width: u32, height: u32, counts: Vec<u32>) -> Self {
        Self {
            counts: DensityCountGrid::new(GridSize::new(width, height), counts),
        }
    }

    /// Returns the grid width in time bins.
    pub fn width(&self) -> u32 {
        self.counts.width()
    }

    /// Returns the grid height in lane bins.
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

    /// Returns the total number of binned events.
    pub fn total_count(&self) -> u64 {
        self.counts.total_count()
    }
}

/// Errors returned by GPU timeline-density compute and readback.
#[derive(Debug)]
pub enum GpuTimelineDensityError {
    EventCountTooLarge { event_count: usize },
    TimeRangeTooWide { span: u64 },
    MissingReadbackCounts,
    BufferMap(wgpu::BufferAsyncError),
    BufferMapCallbackDropped(RecvError),
    DevicePoll(wgpu::PollError),
    InvalidConfiguration(&'static str),
}

impl fmt::Display for GpuTimelineDensityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventCountTooLarge { event_count } => {
                write!(f, "event count {event_count} exceeds u32::MAX")
            }
            Self::TimeRangeTooWide { span } => write!(
                f,
                "timeline range span {span} exceeds portable u32 shader binning support"
            ),
            Self::MissingReadbackCounts => write!(
                f,
                "GPU timeline-density readback counts were requested but not returned"
            ),
            Self::BufferMap(err) => write!(f, "failed to map GPU timeline-density readback: {err}"),
            Self::BufferMapCallbackDropped(err) => {
                write!(
                    f,
                    "GPU timeline-density readback callback did not run: {err}"
                )
            }
            Self::DevicePoll(err) => write!(f, "failed while polling GPU device: {err}"),
            Self::InvalidConfiguration(reason) => {
                write!(f, "invalid timeline density configuration: {reason}")
            }
        }
    }
}

impl Error for GpuTimelineDensityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EventCountTooLarge { .. }
            | Self::TimeRangeTooWide { .. }
            | Self::MissingReadbackCounts => None,
            Self::BufferMap(err) => Some(err),
            Self::BufferMapCallbackDropped(err) => Some(err),
            Self::DevicePoll(err) => Some(err),
            Self::InvalidConfiguration(_) => None,
        }
    }
}

impl From<GpuDensityReadbackError> for GpuTimelineDensityError {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimelineDensityComputeConfig {
    pub time_range: U64Range,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
}

/// Bins timeline events into a 2D density grid using a WGPU compute shader.
///
/// This correctness path returns counts only. Timeline rendering, brushing, and
/// row evidence remain deferred to later timeline milestones.
pub async fn gpu_timeline_density(
    context: &ComputeContext,
    events: &[TimelineEventRecord],
    time_range: U64Range,
    lane_count: u32,
    width: u32,
    height: u32,
) -> Result<GpuTimelineDensityGrid, GpuTimelineDensityError> {
    gpu_timeline_density_on_device(
        context.device(),
        context.queue(),
        events,
        time_range,
        lane_count,
        width,
        height,
    )
    .await
}

/// Bins timeline events using an existing WGPU device and queue.
pub async fn gpu_timeline_density_on_device(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    events: &[TimelineEventRecord],
    time_range: U64Range,
    lane_count: u32,
    width: u32,
    height: u32,
) -> Result<GpuTimelineDensityGrid, GpuTimelineDensityError> {
    validate_timeline_configuration(time_range, lane_count, width, height)?;
    let event_count =
        u32::try_from(events.len()).map_err(|_| GpuTimelineDensityError::EventCountTooLarge {
            event_count: events.len(),
        })?;

    let grid_bin_count = (width as usize) * (height as usize);
    if event_count == 0 {
        return Ok(GpuTimelineDensityGrid::new(
            width,
            height,
            vec![0; grid_bin_count],
        ));
    }

    let compute_config = TimelineDensityComputeConfig {
        time_range,
        lane_count,
        grid_width: width,
        grid_height: height,
    };
    let compute_output = dispatch_timeline_density(device, queue, events, compute_config, true)?;
    let counts = compute_output
        .counts
        .ok_or(GpuTimelineDensityError::MissingReadbackCounts)?;
    Ok(GpuTimelineDensityGrid::new(width, height, counts))
}

pub(crate) struct TimelineDensityComputeOutput {
    pub count_buffer: wgpu::Buffer,
    pub counts: Option<Vec<u32>>,
}

pub(crate) fn dispatch_timeline_density(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    events: &[TimelineEventRecord],
    config: TimelineDensityComputeConfig,
    readback_counts: bool,
) -> Result<TimelineDensityComputeOutput, GpuTimelineDensityError> {
    validate_timeline_configuration(
        config.time_range,
        config.lane_count,
        config.grid_width,
        config.grid_height,
    )?;
    let event_count =
        u32::try_from(events.len()).map_err(|_| GpuTimelineDensityError::EventCountTooLarge {
            event_count: events.len(),
        })?;
    let timeline_span = timeline_span_u32(config.time_range)?;

    let grid_bin_count = (config.grid_width as usize) * (config.grid_height as usize);
    let packed_events = pack_events(events, config.time_range)?;
    let event_buffer = create_storage_upload_buffer(
        device,
        queue,
        "RawScope Timeline Event Buffer",
        bytemuck::cast_slice(&packed_events),
    );

    let output_size_bytes = (grid_bin_count * std::mem::size_of::<u32>()) as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Timeline Density Output Buffer"),
        size: output_size_bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    clear_output_buffer(queue, &output_buffer, output_size_bytes);

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("RawScope Timeline Density Readback Buffer"),
        size: output_size_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RawScope Timeline Density Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });
    let bind_group_layout =
        create_density_bind_group_layout(device, "RawScope Timeline Density Bind Group Layout");
    let pipeline = create_density_compute_pipeline(
        device,
        "RawScope Timeline Density Pipeline Layout",
        "RawScope Timeline Density Pipeline",
        &bind_group_layout,
        &shader,
    );
    let dispatch_chunks = timeline_dispatch_chunks(event_count);
    let dispatch_config = TimelineDispatchConfig {
        density: config,
        time_span: timeline_span,
    };
    let dispatch_bind_groups = create_dispatch_bind_groups(
        device,
        queue,
        &bind_group_layout,
        &event_buffer,
        &output_buffer,
        dispatch_config,
        &dispatch_chunks,
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("RawScope Timeline Density Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RawScope Timeline Density Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        for dispatch_bind_group in &dispatch_bind_groups {
            let chunk_workgroup_count = dispatch_bind_group
                .chunk
                .event_count
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

    Ok(TimelineDensityComputeOutput {
        count_buffer: output_buffer,
        counts,
    })
}

fn validate_timeline_configuration(
    time_range: U64Range,
    lane_count: u32,
    width: u32,
    height: u32,
) -> Result<(), GpuTimelineDensityError> {
    if lane_count == 0 {
        return Err(GpuTimelineDensityError::InvalidConfiguration(
            "lane count must be positive",
        ));
    }
    if width == 0 || height == 0 {
        return Err(GpuTimelineDensityError::InvalidConfiguration(
            "grid dimensions must be positive",
        ));
    }
    if time_range.span() == 0 {
        return Err(GpuTimelineDensityError::InvalidConfiguration(
            "time range span must be positive",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimelineDispatchConfig {
    density: TimelineDensityComputeConfig,
    time_span: u32,
}

struct TimelineDispatchChunk {
    event_start: u32,
    event_count: u32,
}

struct TimelineDispatchBindGroup {
    chunk: TimelineDispatchChunk,
    _params_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

fn timeline_dispatch_chunks(event_count: u32) -> Vec<TimelineDispatchChunk> {
    let mut chunks = Vec::new();
    let mut event_start = 0;

    while event_start < event_count {
        let remaining_event_count = event_count - event_start;
        let chunk_event_count = remaining_event_count.min(MAX_EVENTS_PER_DISPATCH);
        chunks.push(TimelineDispatchChunk {
            event_start,
            event_count: chunk_event_count,
        });
        event_start += chunk_event_count;
    }

    chunks
}

fn create_dispatch_bind_groups(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    event_buffer: &wgpu::Buffer,
    output_buffer: &wgpu::Buffer,
    config: TimelineDispatchConfig,
    dispatch_chunks: &[TimelineDispatchChunk],
) -> Vec<TimelineDispatchBindGroup> {
    dispatch_chunks
        .iter()
        .map(|chunk| {
            let params = TimelineParams::new(
                config.time_span,
                config.density.lane_count,
                config.density.grid_width,
                config.density.grid_height,
                chunk.event_start,
                chunk.event_count,
            );
            let params_buffer = create_uniform_upload_buffer(
                device,
                queue,
                "RawScope Timeline Params Buffer",
                bytemuck::bytes_of(&params),
            );
            let bind_group = create_density_bind_group(
                device,
                "RawScope Timeline Density Bind Group",
                bind_group_layout,
                event_buffer,
                &params_buffer,
                output_buffer,
            );

            TimelineDispatchBindGroup {
                chunk: TimelineDispatchChunk {
                    event_start: chunk.event_start,
                    event_count: chunk.event_count,
                },
                _params_buffer: params_buffer,
                bind_group,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rawscope_core::U64Range;

    use super::{
        timeline_dispatch_chunks, validate_timeline_configuration, GpuTimelineDensityError,
        MAX_EVENTS_PER_DISPATCH,
    };

    #[test]
    fn dispatch_chunks_keep_each_dispatch_within_wgpu_limit() {
        let event_count = MAX_EVENTS_PER_DISPATCH + 10;

        let chunks = timeline_dispatch_chunks(event_count);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].event_start, 0);
        assert_eq!(chunks[0].event_count, MAX_EVENTS_PER_DISPATCH);
        assert_eq!(chunks[1].event_start, MAX_EVENTS_PER_DISPATCH);
        assert_eq!(chunks[1].event_count, 10);
    }

    #[test]
    fn invalid_timeline_configuration_is_rejected_before_empty_fast_path() {
        assert!(matches!(
            validate_timeline_configuration(U64Range::new(0, 1), 0, 0, 0),
            Err(GpuTimelineDensityError::InvalidConfiguration(_))
        ));
    }
}
