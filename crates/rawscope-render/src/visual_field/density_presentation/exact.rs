//! Exact-cell presentation contract.

use crate::PlotRectPx;
use rawscope_evidence::ScatterDensityPresentation;

use super::DensityPresentation;

pub(super) const EXACT_PRESENTATION: ScatterDensityPresentation =
    ScatterDensityPresentation::ExactCells;

impl DensityPresentation {
    /// Encodes one density presentation pass clipped to the physical plot.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
    ) {
        self.render_blended(encoder, target_view, plot_rect, true, 1.0);
    }

    pub fn render_blended(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        plot_rect: PlotRectPx,
        clear: bool,
        opacity: f32,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RawScope Scatter Density Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.015,
                            g: 0.025,
                            b: 0.035,
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

        render_pass.set_pipeline(&self.pipeline);
        let opacity = f64::from(opacity.clamp(0.0, 1.0));
        render_pass.set_blend_constant(wgpu::Color {
            r: opacity,
            g: opacity,
            b: opacity,
            a: opacity,
        });
        render_pass.set_bind_group(
            0,
            &self.bind_groups[self.gpu_state.active_count_buffer_index()],
            &[],
        );
        render_pass.set_viewport(
            plot_rect.x as f32,
            plot_rect.y as f32,
            plot_rect.width as f32,
            plot_rect.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(plot_rect.x, plot_rect.y, plot_rect.width, plot_rect.height);
        render_pass.draw(0..3, 0..1);
    }
}
