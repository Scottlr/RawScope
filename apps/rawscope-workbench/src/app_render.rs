//! Frame rendering for RawScope density views.

use rawscope_gpu::ClearFrameStatus;
use tracing::error;
use winit::event_loop::ActiveEventLoop;

use crate::{app::WorkbenchApp, demo::DemoMode};

impl WorkbenchApp {
    pub(crate) fn render(&mut self, event_loop: &ActiveEventLoop) {
        let screen_size = self.screen_size();
        let render_status = {
            let Some(gpu) = self.gpu.as_mut() else {
                return;
            };

            match self.demo_mode {
                DemoMode::Scatter => {
                    let Some(scatter_density_renderer) = self.scatter.density_renderer.as_ref()
                    else {
                        return;
                    };
                    let Some(scatter_brush_overlay_renderer) =
                        self.scatter_brush_overlay_renderer.as_ref()
                    else {
                        return;
                    };
                    let brush_screen_rect = self
                        .scatter
                        .active_brush_drag
                        .map(|drag| drag.screen_rect)
                        .or_else(|| {
                            let viewport = self.scatter.viewport?;
                            let selection = self.scatter.active_brush_selection?;
                            selection.project_to_screen(viewport, screen_size)
                        });

                    gpu.render_frame(|_device, queue, target_view, encoder| {
                        scatter_density_renderer.render(encoder, target_view);
                        scatter_brush_overlay_renderer.render(
                            queue,
                            encoder,
                            target_view,
                            brush_screen_rect,
                            screen_size,
                        );
                    })
                }
                DemoMode::Timeline => {
                    let Some(timeline_density_renderer) = self.timeline.density_renderer.as_ref()
                    else {
                        return;
                    };
                    let Some(scatter_brush_overlay_renderer) =
                        self.scatter_brush_overlay_renderer.as_ref()
                    else {
                        return;
                    };
                    let brush_screen_rect = self
                        .timeline
                        .active_brush_drag
                        .map(|drag| drag.screen_rect)
                        .or_else(|| {
                            let viewport = self.timeline.viewport?;
                            let selection = self.timeline.active_brush_selection?;
                            selection.project_to_screen(viewport, screen_size)
                        });

                    gpu.render_frame(|_device, queue, target_view, encoder| {
                        timeline_density_renderer.render(encoder, target_view);
                        scatter_brush_overlay_renderer.render(
                            queue,
                            encoder,
                            target_view,
                            brush_screen_rect,
                            screen_size,
                        );
                    })
                }
            }
        };

        match render_status {
            Ok(ClearFrameStatus::Presented) => {
                self.update_window_title();
            }
            Ok(ClearFrameStatus::SkippedZeroSizedSurface | ClearFrameStatus::SkippedOccluded) => {}
            Ok(ClearFrameStatus::SkippedTimeout | ClearFrameStatus::Reconfigured) => {
                self.request_redraw();
            }
            Err(err) => {
                error!(error = %err, "failed to render workbench frame");
                event_loop.exit();
            }
        }
    }
}
