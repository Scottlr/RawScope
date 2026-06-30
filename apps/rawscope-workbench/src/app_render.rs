//! Frame rendering for the workbench scatter-density demo.

use std::time::Instant;

use rawscope_gpu::ClearFrameStatus;
use rawscope_render::BrushScreenSize;
use tracing::{error, info};
use winit::event_loop::ActiveEventLoop;

use crate::app::WorkbenchApp;

const FRAME_DIAGNOSTIC_INTERVAL: u64 = 5_000;

impl WorkbenchApp {
    pub(crate) fn render(&mut self, event_loop: &ActiveEventLoop) {
        let frame_start = Instant::now();
        let render_status = {
            let Some(gpu) = self.gpu.as_mut() else {
                return;
            };
            let Some(scatter_density_renderer) = self.scatter_density_renderer.as_ref() else {
                return;
            };
            let Some(scatter_brush_overlay_renderer) = self.scatter_brush_overlay_renderer.as_ref()
            else {
                return;
            };
            let screen_size = self
                .window
                .as_ref()
                .map(|window| {
                    let size = window.inner_size();
                    BrushScreenSize::new(size.width as f32, size.height as f32)
                })
                .unwrap_or_else(|| BrushScreenSize::new(0.0, 0.0));
            let brush_screen_rect =
                self.active_brush_drag
                    .map(|drag| drag.screen_rect)
                    .or_else(|| {
                        let viewport = self.viewport?;
                        let selection = self.active_brush_selection?;
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
        };

        match render_status {
            Ok(ClearFrameStatus::Presented) => {
                self.redraw_count += 1;
                self.latest_frame_cpu_duration = frame_start.elapsed();
                let should_log_frame = self.redraw_count.is_multiple_of(FRAME_DIAGNOSTIC_INTERVAL);
                if should_log_frame {
                    info!(
                        redraw_count = self.redraw_count,
                        frame_cpu_ms = frame_start.elapsed().as_secs_f64() * 1000.0,
                        "RawScope frame diagnostics"
                    );
                }
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
