//! GPU timeline-density compute reference for correctness checks.

use std::{
    error::Error,
    fmt,
    sync::mpsc::{self, RecvError},
};

use bytemuck::{Pod, Zeroable};
use rawscope_core::U64Range;
use rawscope_data::SyntheticEventRecord;
use rawscope_gpu::ComputeContext;

const WORKGROUP_SIZE: u32 = 64;
const MAX_DISPATCH_WORKGROUPS_PER_DIMENSION: u32 = 65_535;
const MAX_EVENTS_PER_DISPATCH: u32 = WORKGROUP_SIZE * MAX_DISPATCH_WORKGROUPS_PER_DIMENSION;
const EMPTY_BUFFER_SIZE_BYTES: u64 = 4;
const SHADER_SOURCE: &str = include_str!("shaders/timeline_density.wgsl");

/// Flattened GPU timeline-density counts for a 2D grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTimelineDensityGrid {
    width: u32,
    height: u32,
    counts: Vec<u32>,
}

impl GpuTimelineDensityGrid {
    /// Creates a flattened GPU timeline count grid.
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

    /// Returns the grid width in time bins.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the grid height in lane bins.
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

    /// Returns the total number of binned events.
    pub fn total_count(&self) -> u64 {
        self.counts.iter().map(|count| *count as u64).sum()
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
    events: &[SyntheticEventRecord],
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
    events: &[SyntheticEventRecord],
    time_range: U64Range,
    lane_count: u32,
    width: u32,
    height: u32,
) -> Result<GpuTimelineDensityGrid, GpuTimelineDensityError> {
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
    events: &[SyntheticEventRecord],
    config: TimelineDensityComputeConfig,
    readback_counts: bool,
) -> Result<TimelineDensityComputeOutput, GpuTimelineDensityError> {
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
    let bind_group_layout = create_bind_group_layout(device);
    let pipeline = create_compute_pipeline(device, &bind_group_layout, &shader);
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuTimelineEvent {
    timestamp_offset: u32,
    lane: u32,
    is_before_time_range: u32,
    is_after_time_range: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TimelineParams {
    time_span: u32,
    lane_count: u32,
    grid_width: u32,
    grid_height: u32,
    event_start: u32,
    dispatch_event_count: u32,
}

impl TimelineParams {
    fn new(
        time_span: u32,
        lane_count: u32,
        grid_width: u32,
        grid_height: u32,
        event_start: u32,
        dispatch_event_count: u32,
    ) -> Self {
        Self {
            time_span,
            lane_count,
            grid_width,
            grid_height,
            event_start,
            dispatch_event_count,
        }
    }
}

fn pack_events(
    events: &[SyntheticEventRecord],
    time_range: U64Range,
) -> Result<Vec<GpuTimelineEvent>, GpuTimelineDensityError> {
    events
        .iter()
        .map(|event| pack_event(event, time_range))
        .collect()
}

fn pack_event(
    event: &SyntheticEventRecord,
    time_range: U64Range,
) -> Result<GpuTimelineEvent, GpuTimelineDensityError> {
    let timestamp_is_before_range = event.timestamp < time_range.min;
    let timestamp_is_after_range = event.timestamp > time_range.max;
    let timestamp_offset = if timestamp_is_before_range {
        0
    } else {
        let offset = event.timestamp.saturating_sub(time_range.min);
        u32::try_from(offset)
            .map_err(|_| GpuTimelineDensityError::TimeRangeTooWide { span: offset })?
    };

    Ok(GpuTimelineEvent {
        timestamp_offset,
        lane: event.lane,
        is_before_time_range: u32::from(timestamp_is_before_range),
        is_after_time_range: u32::from(timestamp_is_after_range),
    })
}

fn timeline_span_u32(time_range: U64Range) -> Result<u32, GpuTimelineDensityError> {
    let span = time_range.span();
    u32::try_from(span).map_err(|_| GpuTimelineDensityError::TimeRangeTooWide { span })
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
            let bind_group = create_bind_group(
                device,
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

fn clear_output_buffer(queue: &wgpu::Queue, buffer: &wgpu::Buffer, output_size_bytes: u64) {
    let zeroed_output = vec![0_u8; output_size_bytes as usize];
    queue.write_buffer(buffer, 0, &zeroed_output);
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Timeline Density Bind Group Layout"),
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
    event_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    output_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Timeline Density Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: event_buffer.as_entire_binding(),
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
        label: Some("RawScope Timeline Density Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("RawScope Timeline Density Pipeline"),
        layout: Some(&pipeline_layout),
        module: shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn readback_counts_from_buffer(
    device: &wgpu::Device,
    readback_buffer: &wgpu::Buffer,
    grid_bin_count: usize,
) -> Result<Vec<u32>, GpuTimelineDensityError> {
    let readback_slice = readback_buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    readback_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(GpuTimelineDensityError::DevicePoll)?;

    receiver
        .recv()
        .map_err(GpuTimelineDensityError::BufferMapCallbackDropped)?
        .map_err(GpuTimelineDensityError::BufferMap)?;

    let counts = {
        let mapped = readback_slice.get_mapped_range();
        bytemuck::cast_slice::<u8, u32>(&mapped)[..grid_bin_count].to_vec()
    };
    readback_buffer.unmap();

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use rawscope_core::{RowId, U64Range};
    use rawscope_data::{SyntheticEventRecord, SyntheticEventType};

    use super::{
        pack_event, timeline_dispatch_chunks, timeline_span_u32, GpuTimelineDensityError,
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
    fn pack_event_marks_timestamps_outside_range() {
        let time_range = U64Range::new(100, 200);
        let before = pack_event(&event(0, 99, 0), time_range).unwrap();
        let inside = pack_event(&event(1, 150, 0), time_range).unwrap();
        let after = pack_event(&event(2, 201, 0), time_range).unwrap();

        assert_eq!(before.is_before_time_range, 1);
        assert_eq!(before.is_after_time_range, 0);
        assert_eq!(inside.timestamp_offset, 50);
        assert_eq!(inside.is_before_time_range, 0);
        assert_eq!(inside.is_after_time_range, 0);
        assert_eq!(after.is_before_time_range, 0);
        assert_eq!(after.is_after_time_range, 1);
    }

    #[test]
    fn timeline_span_rejects_ranges_wider_than_u32() {
        let err = timeline_span_u32(U64Range::new(0, u32::MAX as u64 + 1))
            .expect_err("wide ranges should be rejected");

        assert!(matches!(
            err,
            GpuTimelineDensityError::TimeRangeTooWide { .. }
        ));
    }

    fn event(row_id: u64, timestamp: u64, lane: u32) -> SyntheticEventRecord {
        SyntheticEventRecord {
            row_id: RowId(row_id),
            timestamp,
            lane,
            value: 1.0,
            event_type: SyntheticEventType::Background,
        }
    }
}
