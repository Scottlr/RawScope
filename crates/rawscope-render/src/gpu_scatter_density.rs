//! GPU scatter-density correctness entry points backed by resident resources.

use std::{error::Error, fmt, sync::mpsc::RecvError};

use rawscope_core::{DensityCountGrid, F32Range, GridSize};
use rawscope_data::{FilterMask, FilterRevision, ScatterPointRecord};
use rawscope_gpu::ComputeContext;

use crate::{
    gpu_density_pipeline::GpuDensityReadbackError, DensityReadbackPolicy, ScatterDensityGpuState,
    ScatterDensityRendererConfig, ScatterDensityUpdate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuScatterDensityGrid {
    counts: DensityCountGrid,
}

impl GpuScatterDensityGrid {
    pub fn new(width: u32, height: u32, counts: Vec<u32>) -> Self {
        Self {
            counts: DensityCountGrid::new(GridSize::new(width, height), counts),
        }
    }
    pub fn width(&self) -> u32 {
        self.counts.width()
    }
    pub fn height(&self) -> u32 {
        self.counts.height()
    }
    pub fn counts(&self) -> &[u32] {
        self.counts.counts()
    }
    pub fn count(&self, x: u32, y: u32) -> u32 {
        self.counts.count(x, y)
    }
    pub fn total_count(&self) -> u64 {
        self.counts.total_count()
    }
}

#[derive(Debug)]
pub enum GpuScatterDensityError {
    PointCountTooLarge {
        point_count: usize,
    },
    FilterMaskLengthMismatch {
        point_count: usize,
        mask_len: usize,
    },
    MissingReadbackCounts,
    ReadbackInProgress,
    BufferMap(wgpu::BufferAsyncError),
    BufferMapCallbackDropped(RecvError),
    BufferMapCallbackTimedOut,
    ReadbackSizeOverflow,
    ReadbackBufferTooSmall {
        expected_bytes: usize,
        actual_bytes: usize,
    },
    DevicePoll(wgpu::PollError),
}

impl fmt::Display for GpuScatterDensityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointCountTooLarge { point_count } => {
                write!(f, "point count {point_count} exceeds u32::MAX")
            }
            Self::FilterMaskLengthMismatch {
                point_count,
                mask_len,
            } => write!(
                f,
                "scatter filter mask length {mask_len} does not match point count {point_count}"
            ),
            Self::MissingReadbackCounts => write!(
                f,
                "GPU scatter-density full readback was requested but not returned"
            ),
            Self::ReadbackInProgress => {
                write!(f, "GPU scatter-density full readback is already in progress")
            }
            Self::BufferMap(error) => {
                write!(f, "failed to map GPU scatter-density readback: {error}")
            }
            Self::BufferMapCallbackDropped(error) => write!(
                f,
                "GPU scatter-density readback callback did not run: {error}"
            ),
            Self::BufferMapCallbackTimedOut => {
                write!(
                    f,
                    "GPU scatter-density readback callback exceeded its bounded wait"
                )
            }
            Self::ReadbackSizeOverflow => {
                write!(f, "GPU scatter-density readback size overflowed usize")
            }
            Self::ReadbackBufferTooSmall {
                expected_bytes,
                actual_bytes,
            } => write!(
                f,
                "GPU scatter-density readback buffer has {actual_bytes} bytes; expected {expected_bytes}"
            ),
            Self::DevicePoll(error) => write!(f, "failed while polling GPU device: {error}"),
        }
    }
}

impl Error for GpuScatterDensityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BufferMap(error) => Some(error),
            Self::BufferMapCallbackDropped(error) => Some(error),
            Self::DevicePoll(error) => Some(error),
            Self::PointCountTooLarge { .. }
            | Self::FilterMaskLengthMismatch { .. }
            | Self::MissingReadbackCounts
            | Self::ReadbackInProgress => None,
            Self::BufferMapCallbackTimedOut => None,
            Self::ReadbackSizeOverflow | Self::ReadbackBufferTooSmall { .. } => None,
        }
    }
}

impl From<GpuDensityReadbackError> for GpuScatterDensityError {
    fn from(error: GpuDensityReadbackError) -> Self {
        match error {
            GpuDensityReadbackError::BufferMap(error) => Self::BufferMap(error),
            GpuDensityReadbackError::BufferMapCallbackDropped(error) => {
                Self::BufferMapCallbackDropped(error)
            }
            GpuDensityReadbackError::BufferMapCallbackTimedOut => Self::BufferMapCallbackTimedOut,
            GpuDensityReadbackError::ReadbackSizeOverflow => Self::ReadbackSizeOverflow,
            GpuDensityReadbackError::ReadbackBufferTooSmall {
                expected_bytes,
                actual_bytes,
            } => Self::ReadbackBufferTooSmall {
                expected_bytes,
                actual_bytes,
            },
            GpuDensityReadbackError::DevicePoll(error) => Self::DevicePoll(error),
        }
    }
}

pub fn gpu_scatter_density(
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
}

pub fn gpu_scatter_density_on_device(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    points: &[ScatterPointRecord],
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> Result<GpuScatterDensityGrid, GpuScatterDensityError> {
    let config = ScatterDensityRendererConfig::new(x_range, y_range, width, height);
    let mut state = ScatterDensityGpuState::new(device, queue, points, config, 0)?;
    let output = state.update_with_output(
        device,
        queue,
        ScatterDensityUpdate {
            config,
            readback: DensityReadbackPolicy::FullCounts,
        },
    )?;
    let counts = output
        .counts
        .ok_or(GpuScatterDensityError::MissingReadbackCounts)?;
    Ok(GpuScatterDensityGrid::new(width, height, counts))
}

#[allow(clippy::too_many_arguments)]
pub fn gpu_scatter_density_masked(
    context: &ComputeContext,
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    revision: FilterRevision,
    x_range: F32Range,
    y_range: F32Range,
    width: u32,
    height: u32,
) -> Result<GpuScatterDensityGrid, GpuScatterDensityError> {
    let device = context.device();
    let queue = context.queue();
    let config = ScatterDensityRendererConfig::new(x_range, y_range, width, height);
    let mut state = ScatterDensityGpuState::new(device, queue, points, config, 0)?;
    state.update_filter_mask(queue, mask.as_gpu_u32_slice(), revision)?;
    let output = state.update_with_output(
        device,
        queue,
        ScatterDensityUpdate {
            config,
            readback: DensityReadbackPolicy::FullCounts,
        },
    )?;
    let counts = output
        .counts
        .ok_or(GpuScatterDensityError::MissingReadbackCounts)?;
    Ok(GpuScatterDensityGrid::new(width, height, counts))
}
