//! Static, undirected ridge-mark render pipeline.

use bytemuck::{Pod, Zeroable};

use super::RidgeFieldGpuResources;

const RIDGE_RENDER_SHADER: &str = include_str!("../shaders/visual_field_ridge_render.wgsl");

/// Uniforms for the static overlay. Plot aspect is applied at draw time so a
/// viewport change never schedules a new ridge compute generation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct RidgeRenderParams {
    pub grid_width: u32,
    pub grid_height: u32,
    pub _padding: [u32; 2],
    pub plot_scale_x: f32,
    pub plot_scale_y: f32,
    pub mark_length: f32,
    pub opacity: f32,
}

pub struct RidgeRenderPipeline {
    pub layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
}

pub fn ridge_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Ridge Render Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<RidgeRenderParams>() as u64,
                    ),
                },
                count: None,
            },
        ],
    })
}

pub fn ridge_render_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    resources: &RidgeFieldGpuResources,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Ridge Render Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: resources.cells.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

pub fn ridge_render_pipeline(
    device: &wgpu::Device,
    layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> RidgeRenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RawScope Ridge Render Shader"),
        source: wgpu::ShaderSource::Wgsl(RIDGE_RENDER_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Ridge Render Pipeline Layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RawScope Ridge Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    RidgeRenderPipeline { layout, pipeline }
}
