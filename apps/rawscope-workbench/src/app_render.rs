//! Frame rendering for RawScope density views.

use egui_wgpu::ScreenDescriptor;
use rawscope_gpu::ClearFrameStatus;
use tracing::error;
use winit::event_loop::ActiveEventLoop;

use crate::{app::WorkbenchApp, demo::DemoMode};

impl WorkbenchApp {
    pub(crate) fn render(&mut self, event_loop: &ActiveEventLoop) {
        let screen_size = self.screen_size();
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        if self.egui_state.is_none() {
            return;
        }

        let egui_context = self.egui_context.clone();
        let raw_input = {
            let Some(egui_state) = self.egui_state.as_mut() else {
                return;
            };
            egui_state.take_egui_input(window.as_ref())
        };
        let mut ui_actions = crate::ui_controls::UiActions::default();
        let full_output = egui_context.run_ui(raw_input, |ui| {
            ui_actions = self.show_ui(ui);
        });
        if let Some(egui_state) = self.egui_state.as_mut() {
            egui_state.handle_platform_output_with_event_loop(
                window.as_ref(),
                event_loop,
                full_output.platform_output,
            );
        }
        self.apply_ui_actions(ui_actions);

        let pixels_per_point = egui_winit::pixels_per_point(&egui_context, window.as_ref());
        let paint_jobs = egui_context.tessellate(full_output.shapes, pixels_per_point);
        let textures_delta = full_output.textures_delta;
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [screen_size.width as u32, screen_size.height as u32],
            pixels_per_point,
        };
        let render_status = {
            let Some(gpu) = self.gpu.as_mut() else {
                return;
            };
            let Some(egui_renderer) = self.egui_renderer.as_mut() else {
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

                    gpu.render_frame(|device, queue, target_view, encoder| {
                        scatter_density_renderer.render(encoder, target_view);
                        scatter_brush_overlay_renderer.render(
                            queue,
                            encoder,
                            target_view,
                            brush_screen_rect,
                            screen_size,
                        );

                        for (texture_id, delta) in &textures_delta.set {
                            egui_renderer.update_texture(device, queue, *texture_id, delta);
                        }
                        let _user_command_buffers = egui_renderer.update_buffers(
                            device,
                            queue,
                            encoder,
                            &paint_jobs,
                            &screen_descriptor,
                        );

                        let render_pass =
                            encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                                label: Some("RawScope egui scatter pass"),
                                color_attachments: &[Some(
                                    egui_wgpu::wgpu::RenderPassColorAttachment {
                                        view: target_view,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: egui_wgpu::wgpu::Operations {
                                            load: egui_wgpu::wgpu::LoadOp::Load,
                                            store: egui_wgpu::wgpu::StoreOp::Store,
                                        },
                                    },
                                )],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                                multiview_mask: None,
                            });
                        let mut render_pass = render_pass.forget_lifetime();
                        egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
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

                    gpu.render_frame(|device, queue, target_view, encoder| {
                        timeline_density_renderer.render(encoder, target_view);
                        scatter_brush_overlay_renderer.render(
                            queue,
                            encoder,
                            target_view,
                            brush_screen_rect,
                            screen_size,
                        );

                        for (texture_id, delta) in &textures_delta.set {
                            egui_renderer.update_texture(device, queue, *texture_id, delta);
                        }
                        let _user_command_buffers = egui_renderer.update_buffers(
                            device,
                            queue,
                            encoder,
                            &paint_jobs,
                            &screen_descriptor,
                        );

                        let render_pass =
                            encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                                label: Some("RawScope egui timeline pass"),
                                color_attachments: &[Some(
                                    egui_wgpu::wgpu::RenderPassColorAttachment {
                                        view: target_view,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: egui_wgpu::wgpu::Operations {
                                            load: egui_wgpu::wgpu::LoadOp::Load,
                                            store: egui_wgpu::wgpu::StoreOp::Store,
                                        },
                                    },
                                )],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                                multiview_mask: None,
                            });
                        let mut render_pass = render_pass.forget_lifetime();
                        egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
                    })
                }
            }
        };

        if let Some(egui_renderer) = self.egui_renderer.as_mut() {
            for texture_id in &textures_delta.free {
                egui_renderer.free_texture(texture_id);
            }
        }

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
