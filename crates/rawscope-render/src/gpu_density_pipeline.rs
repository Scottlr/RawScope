//! Shared WGPU plumbing for density compute passes.

use std::{mem::size_of, num::NonZeroU64, sync::mpsc::RecvError, time::Duration};

use rawscope_gpu::ReadbackCompletion;

const EMPTY_BUFFER_SIZE_BYTES: u64 = 4;
const READBACK_WAIT_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(crate) enum GpuDensityReadbackError {
    BufferMap(wgpu::BufferAsyncError),
    #[expect(
        dead_code,
        reason = "retained as a compatibility classification for legacy callback failures"
    )]
    BufferMapCallbackDropped(RecvError),
    BufferMapCallbackTimedOut,
    ReadbackSizeOverflow,
    ReadbackBufferTooSmall {
        expected_bytes: usize,
        actual_bytes: usize,
    },
    DevicePoll(wgpu::PollError),
}

/// Retains a staging buffer and advances one map callback without blocking.
pub(crate) struct DensityReadbackOperation {
    buffer: wgpu::Buffer,
    completion: std::sync::Arc<ReadbackCompletion<(), wgpu::BufferAsyncError>>,
    grid_bin_count: usize,
    terminal: bool,
}

impl DensityReadbackOperation {
    pub(crate) fn start(
        buffer: &wgpu::Buffer,
        grid_bin_count: usize,
    ) -> Result<Self, GpuDensityReadbackError> {
        grid_bin_count
            .checked_mul(size_of::<u32>())
            .ok_or(GpuDensityReadbackError::ReadbackSizeOverflow)?;
        let completion =
            std::sync::Arc::new(ReadbackCompletion::<(), wgpu::BufferAsyncError>::new());
        let callback_completion = std::sync::Arc::clone(&completion);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = callback_completion.publish(result);
            });
        Ok(Self {
            buffer: buffer.clone(),
            completion,
            grid_bin_count,
            terminal: false,
        })
    }

    /// Advances WGPU once without waiting; `None` means the callback is pending.
    pub(crate) fn poll(
        &mut self,
        device: &wgpu::Device,
    ) -> Result<Option<Vec<u32>>, GpuDensityReadbackError> {
        if self.terminal {
            return Ok(None);
        }
        device
            .poll(wgpu::PollType::Poll)
            .map_err(GpuDensityReadbackError::DevicePoll)?;
        let Some(map_result) = self.completion.take() else {
            return Ok(None);
        };
        self.terminal = true;
        map_result.map_err(GpuDensityReadbackError::BufferMap)?;
        let expected_bytes = self
            .grid_bin_count
            .checked_mul(size_of::<u32>())
            .ok_or(GpuDensityReadbackError::ReadbackSizeOverflow)?;
        let slice = self.buffer.slice(..);
        let counts = {
            let mapped = slice.get_mapped_range();
            if mapped.len() < expected_bytes {
                return Err(GpuDensityReadbackError::ReadbackBufferTooSmall {
                    expected_bytes,
                    actual_bytes: mapped.len(),
                });
            }
            bytemuck::cast_slice::<u8, u32>(&mapped)[..self.grid_bin_count].to_vec()
        };
        self.buffer.unmap();
        Ok(Some(counts))
    }
}

pub(crate) fn create_storage_upload_buffer(
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

pub(crate) fn create_uniform_upload_buffer(
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

pub(crate) fn create_density_bind_group_layout(
    device: &wgpu::Device,
    label: &'static str,
    input_min_binding_size: u64,
    params_min_binding_size: u64,
    output_min_binding_size: u64,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(input_min_binding_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(params_min_binding_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(output_min_binding_size),
                },
                count: None,
            },
        ],
    })
}

pub(crate) fn create_density_bind_group(
    device: &wgpu::Device,
    label: &'static str,
    layout: &wgpu::BindGroupLayout,
    input_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    output_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
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

pub(crate) fn create_density_compute_pipeline(
    device: &wgpu::Device,
    pipeline_layout_label: &'static str,
    pipeline_label: &'static str,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::ComputePipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(pipeline_layout_label),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(pipeline_label),
        layout: Some(&pipeline_layout),
        module: shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

pub(crate) fn readback_counts_from_buffer(
    device: &wgpu::Device,
    readback_buffer: &wgpu::Buffer,
    grid_bin_count: usize,
) -> Result<Vec<u32>, GpuDensityReadbackError> {
    let mut operation = DensityReadbackOperation::start(readback_buffer, grid_bin_count)?;
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(READBACK_WAIT_TIMEOUT),
        })
        .map_err(GpuDensityReadbackError::DevicePoll)?;

    operation
        .poll(device)?
        .ok_or(GpuDensityReadbackError::BufferMapCallbackTimedOut)
}
