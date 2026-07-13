//! Generation-safe GPU ridge resources and static presentation helpers.

use std::{error::Error, fmt, sync::Arc};

use bytemuck::{Pod, Zeroable};
use rawscope_analysis::visual_field::{DensityRidgeField, RidgeConfig};
use rawscope_core::GridSize;
use rawscope_gpu::DeviceGeneration;

use super::generation::{VisualFieldGeneration, VisualFieldViewGeneration};

#[path = "ridges_compute.rs"]
mod compute;
#[path = "ridges_presentation.rs"]
mod presentation;
#[path = "ridges_render.rs"]
mod render;

pub use compute::{
    ridge_compute_bind_group, ridge_compute_bind_group_layout, ridge_compute_pipeline,
    ridge_dispatch, RidgeComputePipeline, RidgeGpuParams,
};
pub use presentation::{RidgeInteraction, RidgeReprojection};
pub use render::{
    ridge_render_bind_group, ridge_render_bind_group_layout, ridge_render_pipeline,
    RidgeRenderParams, RidgeRenderPipeline,
};

pub(crate) const RIDGE_WORKGROUP_SIZE: u32 = 64;
const RIDGE_SCRATCH_SCALAR_BUFFERS: u64 = 2;
const RIDGE_CANDIDATE_BYTES_PER_CELL: u64 = 16;
const RIDGE_REDUCTION_BYTES: u64 = 16;
const RIDGE_SCALAR_BYTES_PER_CELL: u64 = 4;

/// Host/WGSL ABI for one normalized ridge strength and undirected tangent.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct GpuRidgeCell {
    pub strength: f32,
    pub tangent_x: f32,
    pub tangent_y: f32,
    pub _padding: f32,
}

/// Encodes the reviewed CPU ridge result into the storage ABI consumed by
/// ridge compute/presentation passes.
pub fn encode_ridge_field(field: &DensityRidgeField) -> Arc<[GpuRidgeCell]> {
    field
        .cells
        .iter()
        .map(|cell| GpuRidgeCell {
            strength: cell.strength,
            tangent_x: cell.tangent_x,
            tangent_y: cell.tangent_y,
            _padding: 0.0,
        })
        .collect::<Vec<_>>()
        .into()
}

/// Checked bytes and dispatch dimensions for one ridge generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RidgeResourcePlan {
    pub grid: GridSize,
    pub ridge_cells_bytes: u64,
    pub smoothing_bytes: u64,
    pub candidate_bytes: u64,
    pub reduction_bytes: u64,
    pub scratch_and_reduction_bytes: u64,
    pub resource_bytes: u64,
    pub dispatch_workgroups_x: u32,
}

/// Truthful pressure result used by coordinators before allocating a ridge
/// generation. No partial field is returned when the selected adapter cannot
/// support the complete intermediate set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RidgePresentationAvailability {
    Available(RidgeResourcePlan),
    Unavailable(RidgeResourceError),
}

pub fn ridge_presentation_availability(
    grid: GridSize,
    limits: &wgpu::Limits,
) -> RidgePresentationAvailability {
    match RidgeResourcePlan::for_grid(grid, limits) {
        Ok(plan) => RidgePresentationAvailability::Available(plan),
        Err(error) => RidgePresentationAvailability::Unavailable(error),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RidgeResourceError {
    ArithmeticOverflow,
    StorageBindingTooLarge { bytes: u64, limit: u64 },
    BufferTooLarge { bytes: u64, limit: u64 },
    DispatchTooLarge { workgroups: u32, limit: u32 },
    SourceGenerationMismatch,
    DeviceGenerationMismatch,
    GridMismatch,
    ConfigMismatch,
}

impl fmt::Display for RidgeResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => formatter.write_str("ridge resource arithmetic overflowed"),
            Self::StorageBindingTooLarge { bytes, limit } => {
                write!(
                    formatter,
                    "ridge storage binding is {bytes} bytes, over limit {limit}"
                )
            }
            Self::BufferTooLarge { bytes, limit } => {
                write!(
                    formatter,
                    "ridge buffer is {bytes} bytes, over limit {limit}"
                )
            }
            Self::DispatchTooLarge { workgroups, limit } => {
                write!(
                    formatter,
                    "ridge dispatch has {workgroups} workgroups, over limit {limit}"
                )
            }
            Self::SourceGenerationMismatch => {
                formatter.write_str("ridge resources belong to a different visual-field generation")
            }
            Self::DeviceGenerationMismatch => {
                formatter.write_str("ridge resources belong to a different device generation")
            }
            Self::GridMismatch => {
                formatter.write_str("ridge resources use different visual-field dimensions")
            }
            Self::ConfigMismatch => {
                formatter.write_str("ridge resource configuration does not match generation")
            }
        }
    }
}

impl Error for RidgeResourceError {}

impl RidgeResourcePlan {
    pub fn for_grid(grid: GridSize, limits: &wgpu::Limits) -> Result<Self, RidgeResourceError> {
        let bins =
            u64::try_from(grid.bin_count()).map_err(|_| RidgeResourceError::ArithmeticOverflow)?;
        let ridge_cells_bytes =
            checked_mul(bins, u64::from(std::mem::size_of::<GpuRidgeCell>() as u32))?;
        let smoothing_bytes = checked_mul(
            checked_mul(bins, RIDGE_SCALAR_BYTES_PER_CELL)?,
            RIDGE_SCRATCH_SCALAR_BUFFERS,
        )?;
        let candidate_bytes = checked_mul(bins, RIDGE_CANDIDATE_BYTES_PER_CELL)?;
        let reduction_bytes = RIDGE_REDUCTION_BYTES;
        let scratch_and_reduction_bytes = smoothing_bytes
            .checked_add(candidate_bytes)
            .and_then(|bytes| bytes.checked_add(reduction_bytes))
            .ok_or(RidgeResourceError::ArithmeticOverflow)?;
        let resource_bytes = ridge_cells_bytes
            .checked_add(scratch_and_reduction_bytes)
            .ok_or(RidgeResourceError::ArithmeticOverflow)?;
        let workgroups = u32::try_from(
            bins.checked_add(u64::from(RIDGE_WORKGROUP_SIZE - 1))
                .ok_or(RidgeResourceError::ArithmeticOverflow)?
                / u64::from(RIDGE_WORKGROUP_SIZE),
        )
        .map_err(|_| RidgeResourceError::ArithmeticOverflow)?;
        let storage_limit = u64::from(limits.max_storage_buffer_binding_size);
        let largest_intermediate = smoothing_bytes.max(candidate_bytes).max(reduction_bytes);
        if ridge_cells_bytes > storage_limit || largest_intermediate > storage_limit {
            return Err(RidgeResourceError::StorageBindingTooLarge {
                bytes: ridge_cells_bytes.max(largest_intermediate),
                limit: storage_limit,
            });
        }
        for bytes in [
            ridge_cells_bytes,
            smoothing_bytes,
            candidate_bytes,
            reduction_bytes,
        ] {
            if bytes > limits.max_buffer_size {
                return Err(RidgeResourceError::BufferTooLarge {
                    bytes,
                    limit: limits.max_buffer_size,
                });
            }
        }
        if workgroups > limits.max_compute_workgroups_per_dimension {
            return Err(RidgeResourceError::DispatchTooLarge {
                workgroups,
                limit: limits.max_compute_workgroups_per_dimension,
            });
        }
        Ok(Self {
            grid,
            ridge_cells_bytes,
            smoothing_bytes,
            candidate_bytes,
            reduction_bytes,
            scratch_and_reduction_bytes,
            resource_bytes,
            dispatch_workgroups_x: workgroups,
        })
    }
}

fn checked_mul(left: u64, right: u64) -> Result<u64, RidgeResourceError> {
    left.checked_mul(right)
        .ok_or(RidgeResourceError::ArithmeticOverflow)
}

/// Device-owned buffers for one immutable source generation and ridge config.
pub struct RidgeFieldGpuResources {
    pub generation: VisualFieldViewGeneration,
    pub device_generation: DeviceGeneration,
    pub grid: GridSize,
    pub config: RidgeConfig,
    pub cells: wgpu::Buffer,
    pub horizontal: wgpu::Buffer,
    pub smoothed: wgpu::Buffer,
    pub candidates: wgpu::Buffer,
    pub reduction: wgpu::Buffer,
    pub scratch_and_reduction_bytes: u64,
    pub resource_bytes: u64,
}

impl RidgeFieldGpuResources {
    pub fn new(
        device: &wgpu::Device,
        source: &VisualFieldGeneration,
        device_generation: DeviceGeneration,
        config: RidgeConfig,
    ) -> Result<Self, RidgeResourceError> {
        let plan = RidgeResourcePlan::for_grid(source.grid(), &device.limits())?;
        let cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Ridge Cells"),
            size: plan.ridge_cells_bytes.max(4),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let storage = |label: &'static str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(4),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let horizontal = storage(
            "RawScope Ridge Horizontal Smoothing",
            plan.smoothing_bytes / 2,
        );
        let smoothed = storage("RawScope Ridge Smoothed Field", plan.smoothing_bytes / 2);
        let candidates = storage("RawScope Ridge Candidates", plan.candidate_bytes);
        let reduction = storage("RawScope Ridge Strength Reduction", plan.reduction_bytes);
        Ok(Self {
            generation: source.view_generation(),
            device_generation,
            grid: source.grid(),
            config,
            cells,
            horizontal,
            smoothed,
            candidates,
            reduction,
            scratch_and_reduction_bytes: plan.scratch_and_reduction_bytes,
            resource_bytes: plan.resource_bytes,
        })
    }

    pub fn rebuild_from_source_generation(
        device: &wgpu::Device,
        source: &VisualFieldGeneration,
        device_generation: DeviceGeneration,
        config: RidgeConfig,
    ) -> Result<Self, RidgeResourceError> {
        Self::new(device, source, device_generation, config)
    }
}

/// Atomic published ridge generation paired with its exact source generation.
pub struct RidgeFieldGeneration {
    pub source: Arc<VisualFieldGeneration>,
    pub config: RidgeConfig,
    pub resources: Arc<RidgeFieldGpuResources>,
}

impl RidgeFieldGeneration {
    pub fn try_new(
        source: Arc<VisualFieldGeneration>,
        config: RidgeConfig,
        resources: Arc<RidgeFieldGpuResources>,
    ) -> Result<Self, RidgeResourceError> {
        validate_ridge_identity(
            source.view_generation(),
            config,
            resources.generation,
            resources.config,
        )?;
        if source.resources().device_generation() != resources.device_generation {
            return Err(RidgeResourceError::DeviceGenerationMismatch);
        }
        if source.grid() != resources.grid {
            return Err(RidgeResourceError::GridMismatch);
        }
        Ok(Self {
            source,
            config,
            resources,
        })
    }
}

pub fn validate_ridge_identity(
    source_generation: VisualFieldViewGeneration,
    config: RidgeConfig,
    resource_generation: VisualFieldViewGeneration,
    resource_config: RidgeConfig,
) -> Result<(), RidgeResourceError> {
    if resource_generation != source_generation {
        return Err(RidgeResourceError::SourceGenerationMismatch);
    }
    if resource_config != config {
        return Err(RidgeResourceError::ConfigMismatch);
    }
    Ok(())
}

/// Applies visible plot aspect to a tangent without introducing directionality.
pub fn transform_ridge_tangent(
    tangent_x: f32,
    tangent_y: f32,
    plot_scale_x: f32,
    plot_scale_y: f32,
) -> Option<[f32; 2]> {
    if !tangent_x.is_finite()
        || !tangent_y.is_finite()
        || !plot_scale_x.is_finite()
        || !plot_scale_y.is_finite()
        || plot_scale_x <= 0.0
        || plot_scale_y <= 0.0
    {
        return None;
    }
    let x = tangent_x * plot_scale_x;
    let y = tangent_y * plot_scale_y;
    let length = x.hypot(y);
    (length.is_finite() && length > f32::EPSILON).then(|| [x / length, y / length])
}

#[cfg(test)]
#[path = "ridges_tests.rs"]
mod tests;
