//! Minimal screen-space brush rectangle overlay rendering.

use bytemuck::{Pod, Zeroable};

use crate::{BrushScreenRect, BrushScreenSize};

const OVERLAY_SHADER_SOURCE: &str = include_str!("shaders/scatter_brush_overlay.wgsl");
const BRUSH_FILL_RGBA: [f32; 4] = [1.0, 0.72, 0.22, 0.18];
const BRUSH_BORDER_RGBA: [f32; 4] = [1.0, 0.86, 0.36, 0.92];
const BRUSH_BORDER_WIDTH_PX: f32 = 2.0;

/// Renders the active scatter brush rectangle over the density view.
///
/// The overlay is intentionally simple: a screen-space amber fill plus border, composited
/// after the density pass. It does not own interaction state or selection summaries.
pub struct ScatterBrushOverlayRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    params_buffer: wgpu::Buffer,
}

impl ScatterBrushOverlayRenderer {
    /// Creates the brush overlay renderer for the given surface format.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RawScope Scatter Brush Overlay Params Buffer"),
            size: std::mem::size_of::<BrushOverlayParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RawScope Scatter Brush Overlay Shader"),
            source: wgpu::ShaderSource::Wgsl(OVERLAY_SHADER_SOURCE.into()),
        });
        let bind_group_layout = create_overlay_bind_group_layout(device);
        let bind_group = create_overlay_bind_group(device, &bind_group_layout, &params_buffer);
        let pipeline = create_overlay_pipeline(device, &bind_group_layout, &shader, surface_format);

        Self {
            pipeline,
            bind_group,
            params_buffer,
        }
    }

    /// Encodes the overlay pass if a brush is active.
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        screen_rect: Option<BrushScreenRect>,
        screen_size: BrushScreenSize,
    ) {
        let Some(screen_rect) = screen_rect else {
            return;
        };
        let Some(params) = BrushOverlayParams::from_screen_rect(screen_rect, screen_size) else {
            return;
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Scatter Brush Overlay Render Pass"),
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
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
struct BrushOverlayParams {
    min_x_px: f32,
    min_y_px: f32,
    max_x_px: f32,
    max_y_px: f32,
    screen_width_px: f32,
    screen_height_px: f32,
    border_width_px: f32,
    _padding: f32,
    fill_rgba: [f32; 4],
    border_rgba: [f32; 4],
}

impl BrushOverlayParams {
    fn from_screen_rect(
        screen_rect: BrushScreenRect,
        screen_size: BrushScreenSize,
    ) -> Option<Self> {
        let screen_has_area = screen_size.width > 0.0 && screen_size.height > 0.0;
        if !screen_has_area {
            return None;
        }

        let rect = clamp_rect_to_screen(screen_rect, screen_size);
        let rect_has_area = rect.max_x > rect.min_x && rect.max_y > rect.min_y;
        if !rect_has_area {
            return None;
        }

        Some(Self {
            min_x_px: rect.min_x,
            min_y_px: rect.min_y,
            max_x_px: rect.max_x,
            max_y_px: rect.max_y,
            screen_width_px: screen_size.width,
            screen_height_px: screen_size.height,
            border_width_px: BRUSH_BORDER_WIDTH_PX,
            _padding: 0.0,
            fill_rgba: BRUSH_FILL_RGBA,
            border_rgba: BRUSH_BORDER_RGBA,
        })
    }
}

fn clamp_rect_to_screen(rect: BrushScreenRect, screen_size: BrushScreenSize) -> BrushScreenRect {
    BrushScreenRect {
        min_x: rect.min_x.clamp(0.0, screen_size.width),
        min_y: rect.min_y.clamp(0.0, screen_size.height),
        max_x: rect.max_x.clamp(0.0, screen_size.width),
        max_y: rect.max_y.clamp(0.0, screen_size.height),
    }
}

fn create_overlay_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("RawScope Scatter Brush Overlay Bind Group Layout"),
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

fn create_overlay_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    params_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("RawScope Scatter Brush Overlay Bind Group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: params_buffer.as_entire_binding(),
        }],
    })
}

fn create_overlay_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("RawScope Scatter Brush Overlay Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RawScope Scatter Brush Overlay Pipeline"),
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
mod tests {
    use super::*;

    #[test]
    fn overlay_params_clamp_brush_rect_to_screen() {
        let screen_rect = BrushScreenRect {
            min_x: -10.0,
            min_y: 5.0,
            max_x: 120.0,
            max_y: 80.0,
        };

        let params =
            BrushOverlayParams::from_screen_rect(screen_rect, BrushScreenSize::new(100.0, 60.0))
                .unwrap();

        assert_eq!(params.min_x_px, 0.0);
        assert_eq!(params.min_y_px, 5.0);
        assert_eq!(params.max_x_px, 100.0);
        assert_eq!(params.max_y_px, 60.0);
    }

    #[test]
    fn overlay_params_skip_zero_sized_screen() {
        let screen_rect = BrushScreenRect {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        };

        assert_eq!(
            BrushOverlayParams::from_screen_rect(screen_rect, BrushScreenSize::new(0.0, 60.0)),
            None
        );
    }
}
