//! GPU focus treatment for settled scatter inspection bins.

use bytemuck::{Pod, Zeroable};

use crate::{BrushScreenRect, BrushScreenSize, PlotRectPx, ScatterInspectionHit};

const OVERLAY_SHADER_SOURCE: &str = include_str!("shaders/scatter_inspection_overlay.wgsl");
const FOCUS_BORDER_WIDTH_PX: f32 = 1.5;
const FOCUS_CROSSHAIR_WIDTH_PX: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionFocusKind {
    Hover,
    Pinned,
}

impl InspectionFocusKind {
    fn shader_id(self) -> u32 {
        match self {
            Self::Hover => 0,
            Self::Pinned => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterInspectionOverlay {
    pub exact_rect: BrushScreenRect,
    pub neighborhood_rect: BrushScreenRect,
    pub focus_kind: InspectionFocusKind,
    pub presentation_alpha: f32,
}

/// Projects a settled inspection bin to local physical plot pixels.
pub fn project_scatter_inspection_overlay(
    hit: &ScatterInspectionHit,
    grid_width: u32,
    grid_height: u32,
    screen_size: BrushScreenSize,
    focus_kind: InspectionFocusKind,
    presentation_alpha: f32,
) -> Option<ScatterInspectionOverlay> {
    if grid_width == 0
        || grid_height == 0
        || hit.bin_x >= grid_width
        || hit.bin_y >= grid_height
        || screen_size.width <= 0.0
        || screen_size.height <= 0.0
    {
        return None;
    }

    let exact_rect = rect_for_bin(hit.bin_x, hit.bin_y, grid_width, grid_height, screen_size);
    let first_x = hit.bin_x.saturating_sub(1);
    let last_x = hit.bin_x.saturating_add(1).min(grid_width - 1);
    let first_y = hit.bin_y.saturating_sub(1);
    let last_y = hit.bin_y.saturating_add(1).min(grid_height - 1);
    let neighborhood_rect = rect_for_bins(
        first_x,
        first_y,
        last_x,
        last_y,
        grid_width,
        grid_height,
        screen_size,
    );

    Some(ScatterInspectionOverlay {
        exact_rect,
        neighborhood_rect,
        focus_kind,
        presentation_alpha: clamp_alpha(presentation_alpha),
    })
}

pub struct ScatterInspectionOverlayRenderer {
    pipeline: wgpu::RenderPipeline,
    hover_bind_group: wgpu::BindGroup,
    hover_params_buffer: wgpu::Buffer,
    pinned_bind_group: wgpu::BindGroup,
    pinned_params_buffer: wgpu::Buffer,
}

impl ScatterInspectionOverlayRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Scatter Inspection Overlay Shader"),
            source: wgpu::ShaderSource::Wgsl(OVERLAY_SHADER_SOURCE.into()),
        });
        let bind_group_layout = create_bind_group_layout(device);
        let hover_params_buffer = create_params_buffer(device, "RawScope Hover Focus Params");
        let pinned_params_buffer = create_params_buffer(device, "RawScope Pinned Focus Params");
        let hover_bind_group = create_bind_group(
            device,
            &bind_group_layout,
            &hover_params_buffer,
            "RawScope Hover Focus Bind Group",
        );
        let pinned_bind_group = create_bind_group(
            device,
            &bind_group_layout,
            &pinned_params_buffer,
            "RawScope Pinned Focus Bind Group",
        );
        let pipeline = create_pipeline(device, &bind_group_layout, &shader, surface_format);

        Self {
            pipeline,
            hover_bind_group,
            hover_params_buffer,
            pinned_bind_group,
            pinned_params_buffer,
        }
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
        pinned: Option<ScatterInspectionOverlay>,
        hovered: Option<ScatterInspectionOverlay>,
    ) {
        let pinned_params =
            pinned.and_then(|overlay| InspectionOverlayParams::from_overlay(overlay, plot_rect));
        let hovered_params =
            hovered.and_then(|overlay| InspectionOverlayParams::from_overlay(overlay, plot_rect));
        if pinned_params.is_none() && hovered_params.is_none() {
            return;
        }

        if let Some(params) = pinned_params {
            queue.write_buffer(&self.pinned_params_buffer, 0, bytemuck::bytes_of(&params));
        }
        if let Some(params) = hovered_params {
            queue.write_buffer(&self.hover_params_buffer, 0, bytemuck::bytes_of(&params));
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Scatter Inspection Focus Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_viewport(
            plot_rect.x as f32,
            plot_rect.y as f32,
            plot_rect.width as f32,
            plot_rect.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
        if pinned_params.is_some() {
            render_pass.set_bind_group(0, &self.pinned_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        if hovered_params.is_some() {
            render_pass.set_bind_group(0, &self.hover_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
struct InspectionOverlayParams {
    exact_rect: [f32; 4],
    neighborhood_rect: [f32; 4],
    plot_origin_px: [f32; 2],
    plot_size_px: [f32; 2],
    border_width_px: f32,
    crosshair_width_px: f32,
    presentation_alpha: f32,
    focus_kind: u32,
    _padding: [u32; 4],
}

impl InspectionOverlayParams {
    fn from_overlay(overlay: ScatterInspectionOverlay, plot_rect: PlotRectPx) -> Option<Self> {
        let screen_size = plot_rect.screen_size();
        if screen_size.width <= 0.0 || screen_size.height <= 0.0 {
            return None;
        }
        Some(Self {
            exact_rect: rect_array(overlay.exact_rect),
            neighborhood_rect: rect_array(overlay.neighborhood_rect),
            plot_origin_px: [plot_rect.x as f32, plot_rect.y as f32],
            plot_size_px: [screen_size.width, screen_size.height],
            border_width_px: FOCUS_BORDER_WIDTH_PX,
            crosshair_width_px: FOCUS_CROSSHAIR_WIDTH_PX,
            presentation_alpha: clamp_alpha(overlay.presentation_alpha),
            focus_kind: overlay.focus_kind.shader_id(),
            _padding: [0; 4],
        })
    }
}

fn rect_for_bin(
    bin_x: u32,
    bin_y: u32,
    grid_width: u32,
    grid_height: u32,
    screen_size: BrushScreenSize,
) -> BrushScreenRect {
    rect_for_bins(
        bin_x,
        bin_y,
        bin_x,
        bin_y,
        grid_width,
        grid_height,
        screen_size,
    )
}

fn rect_for_bins(
    first_x: u32,
    first_y: u32,
    last_x: u32,
    last_y: u32,
    grid_width: u32,
    grid_height: u32,
    screen_size: BrushScreenSize,
) -> BrushScreenRect {
    let min_x = first_x as f32 / grid_width as f32 * screen_size.width;
    let max_x = (last_x + 1) as f32 / grid_width as f32 * screen_size.width;
    let first_screen_row = grid_height - 1 - last_y;
    let last_screen_row = grid_height - 1 - first_y;
    let min_y = first_screen_row as f32 / grid_height as f32 * screen_size.height;
    let max_y = (last_screen_row + 1) as f32 / grid_height as f32 * screen_size.height;
    BrushScreenRect {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

fn rect_array(rect: BrushScreenRect) -> [f32; 4] {
    [rect.min_x, rect.min_y, rect.max_x, rect.max_y]
}

fn clamp_alpha(alpha: f32) -> f32 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn create_params_buffer(device: &wgpu::Device, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<InspectionOverlayParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Scatter Inspection Overlay Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    params_buffer: &wgpu::Buffer,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: params_buffer.as_entire_binding(),
        }],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Scatter Inspection Overlay Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RawScope Scatter Inspection Overlay Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
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
    })
}

#[cfg(test)]
#[path = "scatter_inspection_overlay_tests.rs"]
mod tests;
