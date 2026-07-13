//! WGPU stages for the reviewed density-ridge formula.

use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use super::{RidgeFieldGpuResources, RidgeResourcePlan};

const RIDGE_SHADER_SOURCE: &str = include_str!("../shaders/visual_field_ridge.wgsl");

/// Parameters shared by all ridge compute stages.
///
/// The shader intentionally receives the closed scale and cutoffs as values;
/// it does not infer a scale from viewport size or visual quality tier.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct RidgeGpuParams {
    pub width: u32,
    pub height: u32,
    pub radius: u32,
    pub _padding: u32,
    pub x_step: f32,
    pub y_step: f32,
    pub sigma_squared: f32,
    pub strength_cutoff: f32,
    pub anisotropy_cutoff: f32,
    pub _padding_before_tail: [f32; 3],
    pub _padding_tail: [f32; 4],
}

impl RidgeGpuParams {
    pub fn for_config(
        plan: RidgeResourcePlan,
        config: rawscope_analysis::visual_field::RidgeConfig,
    ) -> Self {
        let width = plan.grid.width();
        let height = plan.grid.height();
        let x_step = if width > 1 {
            1.0 / (width as f32 - 1.0)
        } else {
            1.0
        };
        let y_step = if height > 1 {
            1.0 / (height as f32 - 1.0)
        } else {
            1.0
        };
        Self {
            width,
            height,
            radius: config.scale.radius_cells() as u32,
            _padding: 0,
            x_step,
            y_step,
            sigma_squared: config.scale.effective_sigma_cells() as f32
                * config.scale.effective_sigma_cells() as f32,
            strength_cutoff: f32::from(config.minimum_strength_basis_points) / 10_000.0,
            anisotropy_cutoff: f32::from(config.minimum_anisotropy_basis_points) / 10_000.0,
            _padding_before_tail: [0.0; 3],
            _padding_tail: [0.0; 4],
        }
    }
}

/// The single fixed-layout pipeline contains named entry points for the two
/// smoothing passes, candidate/eigen pass, reduction, and normalization pass.
pub struct RidgeComputePipeline {
    pub layout: wgpu::BindGroupLayout,
    pub stages: [wgpu::ComputePipeline; 5],
}

pub fn ridge_compute_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Ridge Compute Layout"),
        entries: &[
            storage_entry(0, true),
            uniform_entry(1),
            storage_entry(2, false),
            storage_entry(3, false),
            storage_entry(4, false),
            storage_entry(5, false),
            storage_entry(6, false),
        ],
    })
}

pub fn ridge_compute_pipeline(
    device: &wgpu::Device,
    layout: wgpu::BindGroupLayout,
) -> RidgeComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RawScope Ridge Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(RIDGE_SHADER_SOURCE.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Ridge Compute Pipeline Layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let entry_points = [
        "smooth_horizontal",
        "smooth_vertical",
        "candidate",
        "reduce_maximum",
        "normalize",
    ];
    let stages = entry_points.map(|entry_point| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RawScope Ridge Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    });
    RidgeComputePipeline { layout, stages }
}

/// Builds the bind group over shared exact counts and the four ridge
/// intermediates. Coordinates, masks, and row data remain owned by the exact
/// field owner and are not copied into ridge resources.
pub fn ridge_compute_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    count_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    resources: &RidgeFieldGpuResources,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Ridge Compute Bind Group"),
        layout,
        entries: &[
            buffer_entry(0, count_buffer),
            buffer_entry(1, params_buffer),
            buffer_entry(2, &resources.horizontal),
            buffer_entry(3, &resources.smoothed),
            buffer_entry(4, &resources.candidates),
            buffer_entry(5, &resources.reduction),
            buffer_entry(6, &resources.cells),
        ],
    })
}

/// Dispatches all stages only for a settled source update. Pointer/viewport
/// handlers do not call this function; they only reproject the last published
/// ridge generation.
pub fn ridge_dispatch(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &RidgeComputePipeline,
    bind_group: &wgpu::BindGroup,
    resources: &RidgeFieldGpuResources,
    plan: RidgeResourcePlan,
) {
    encoder.clear_buffer(&resources.reduction, 0, Some(plan.reduction_bytes));
    let workgroups = plan.dispatch_workgroups_x;
    for (stage_index, label) in [
        "RawScope Ridge Horizontal Smoothing",
        "RawScope Ridge Vertical Smoothing",
        "RawScope Ridge Candidate And Anisotropy",
        "RawScope Ridge Maximum Reduction",
        "RawScope Ridge Normalization",
    ]
    .into_iter()
    .enumerate()
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(label),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline.stages[stage_index]);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
}

fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(std::mem::size_of::<RidgeGpuParams>() as u64),
        },
        count: None,
    }
}
