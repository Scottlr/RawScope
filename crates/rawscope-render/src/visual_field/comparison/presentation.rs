//! Stable comparison presentation statistics and contract.

/// Statistics disclosed by the comparison visual field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparisonFieldRenderStats {
    pub baseline_total: u64,
    pub active_total: u64,
    pub baseline_recompute_count: u64,
}

use crate::PlotRectPx;

use super::compute::ComparisonFieldRenderer;
use super::resources::difference_bind_group;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    renderer: &ComparisonFieldRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    target_view: &wgpu::TextureView,
    plot_rect: PlotRectPx,
    clear: bool,
    opacity: f32,
) {
    let params = renderer.params();
    queue.write_buffer(&renderer.params_buffer, 0, bytemuck::bytes_of(&params));
    let bind_group = difference_bind_group(
        device,
        &renderer.render_layout,
        renderer
            .baseline
            .count_buffer(renderer.baseline.active_count_buffer_index()),
        renderer
            .active
            .count_buffer(renderer.active.active_count_buffer_index()),
        &renderer.max_abs_buffer,
        &renderer.params_buffer,
        "RawScope Difference Render Bind Group",
    );
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("RawScope Difference Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: if clear {
                    wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.024,
                        b: 0.03,
                        a: 1.0,
                    })
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&renderer.render_pipeline);
    let opacity = f64::from(opacity.clamp(0.0, 1.0));
    pass.set_blend_constant(wgpu::Color {
        r: opacity,
        g: opacity,
        b: opacity,
        a: opacity,
    });
    pass.set_bind_group(0, &bind_group, &[]);
    pass.set_viewport(
        plot_rect.x as f32,
        plot_rect.y as f32,
        plot_rect.width as f32,
        plot_rect.height as f32,
        0.0,
        1.0,
    );
    pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
    pass.draw(0..3, 0..1);
}
