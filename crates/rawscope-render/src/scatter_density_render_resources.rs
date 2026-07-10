//! Scatter-specific render uniforms and bind resources.

use bytemuck::{Pod, Zeroable};
use rawscope_core::F32Range;

use super::ScatterDensityRendererConfig;
use crate::DensityFieldViewport;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(super) struct ScatterDensityRenderParams {
    grid_width: u32,
    grid_height: u32,
    max_bin_count: u32,
    transform_id: u32,
    palette_id: u32,
    presentation_id: u32,
    padding: [u32; 2],
    source_x_min: f32,
    source_x_max: f32,
    source_y_min: f32,
    source_y_max: f32,
    display_x_min: f32,
    display_x_max: f32,
    display_y_min: f32,
    display_y_max: f32,
}

impl ScatterDensityRenderParams {
    pub(super) fn new(
        config: ScatterDensityRendererConfig,
        max_bin_count: u32,
        source: DensityFieldViewport,
        display_x: F32Range,
        display_y: F32Range,
    ) -> Self {
        Self {
            grid_width: source.grid_width,
            grid_height: source.grid_height,
            max_bin_count,
            transform_id: config.encoding.transform.shader_id(),
            palette_id: config.encoding.palette.shader_id(),
            presentation_id: config.presentation.shader_id(),
            padding: [0; 2],
            source_x_min: source.x_range.min,
            source_x_max: source.x_range.max,
            source_y_min: source.y_range.min,
            source_y_max: source.y_range.max,
            display_x_min: display_x.min,
            display_x_max: display_x.max,
            display_y_min: display_y.min,
            display_y_max: display_y.max,
        }
    }
}

pub(super) fn scatter_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Scatter Render Bind Group Layout"),
        entries: &[
            storage_layout_entry(0),
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            storage_layout_entry(2),
        ],
    })
}

fn storage_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(super) fn scatter_render_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    counts: &wgpu::Buffer,
    params: &wgpu::Buffer,
    max_count: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Scatter Render Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: counts.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: max_count.as_entire_binding(),
            },
        ],
    })
}
