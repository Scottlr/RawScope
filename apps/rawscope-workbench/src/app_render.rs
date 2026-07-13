//! Frame rendering for RawScope density views.

use egui_wgpu::ScreenDescriptor;
use rawscope_gpu::ClearFrameStatus;
use tracing::error;
use winit::event_loop::ActiveEventLoop;

use crate::{app::WorkbenchApp, demo::DemoMode};

impl WorkbenchApp {
    pub(crate) fn render(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(err) = self.prepare_scheduled_density() {
            error!(error = %err, "failed to refine interactive scatter density");
        }
        if let Err(err) = self.prepare_scheduled_timeline_density() {
            error!(error = %err, "failed to refine interactive timeline density");
        }
        let transition_frame = self.visual_transition_frame();
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.density_renderer.as_mut())
        {
            renderer.set_transition_progress(gpu.queue(), transition_frame.alpha);
        }
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        if self.egui_state.is_none() {
            return;
        }

        let egui_context = self.egui_context.clone();
        let surface_size = window.inner_size();
        let pixels_per_point = egui_winit::pixels_per_point(&egui_context, window.as_ref());
        let raw_input = {
            let Some(egui_state) = self.egui_state.as_mut() else {
                return;
            };
            egui_state.take_egui_input(window.as_ref())
        };
        let mut ui_output = crate::ui_shell::WorkbenchUiOutput::default();
        let full_output = egui_context.run_ui(raw_input, |ui| {
            ui_output = self.show_ui(
                ui,
                pixels_per_point,
                surface_size.width,
                surface_size.height,
            );
        });
        if let Some(egui_state) = self.egui_state.as_mut() {
            egui_state.handle_platform_output_with_event_loop(
                window.as_ref(),
                event_loop,
                full_output.platform_output,
            );
        }
        self.plot_surface = ui_output.plot_surface;
        self.apply_ui_actions(ui_output.actions);
        self.reassert_plot_cursor();
        self.prepare_scatter_point_reveal();
        self.refresh_point_reveal_emphasis();

        let paint_jobs = egui_context.tessellate(full_output.shapes, pixels_per_point);
        let textures_delta = full_output.textures_delta;
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [surface_size.width, surface_size.height],
            pixels_per_point,
        };
        let render_status = {
            let plot_rect = self.plot_surface.map(|surface| surface.physical_rect);
            let Some(gpu) = self.gpu.as_mut() else {
                return;
            };
            let Some(egui_renderer) = self.egui_renderer.as_mut() else {
                return;
            };

            match (plot_rect, self.demo_mode) {
                (Some(plot_rect), DemoMode::Scatter) => {
                    let Some(scatter_density_renderer) = self.scatter.density_renderer.as_ref()
                    else {
                        return;
                    };
                    let Some(scatter_brush_overlay_renderer) =
                        self.scatter_brush_overlay_renderer.as_ref()
                    else {
                        return;
                    };
                    let point_reveal_renderer = self.point_reveal.renderer.as_ref();
                    let difference_renderer = self.scatter.difference_renderer.as_ref();
                    let density_mode = self.scatter.density_mode;
                    let semantic_modes = transition_frame.semantic_modes;
                    let transition_alpha = transition_frame.alpha;
                    let point_frame = rawscope_render::PointRevealPresentationFrame::from(
                        self.scatter.point_reveal_frame,
                    );
                    let transition_point_frame =
                        rawscope_render::PointRevealPresentationFrame::from_alphas(
                            point_frame.density_alpha,
                            if transition_frame.running {
                                point_frame.point_alpha * transition_alpha
                            } else {
                                point_frame.point_alpha
                            },
                        );
                    let brush_screen_rect = self
                        .scatter
                        .active_brush_drag
                        .map(|drag| drag.screen_rect)
                        .or_else(|| {
                            let viewport = self.scatter.viewport?;
                            let selection = self.scatter.active_brush_selection?;
                            selection.project_to_screen(viewport, plot_rect.screen_size())
                        });
                    let inspection_overlay_inputs =
                        crate::app_scatter_overlays::scatter_inspection_overlay_inputs(
                            &self.scatter_inspection,
                            density_mode,
                            self.inspection_presentation.frame().opacity,
                            self.inspection_presentation
                                .retained_content()
                                .and_then(|content| content.hovered.clone())
                                .as_ref(),
                            plot_rect.screen_size(),
                        );

                    gpu.render_frame(|device, queue, target_view, encoder| {
                        if let Some((from, to)) = semantic_modes {
                            match from {
                                rawscope_render::ScatterDensityMode::AbsoluteDensity => {
                                    scatter_density_renderer.render_blended(
                                        encoder,
                                        target_view,
                                        plot_rect,
                                        true,
                                        point_frame.apply_density(1.0),
                                    );
                                }
                                rawscope_render::ScatterDensityMode::FilteredDifference => {
                                    if let Some(renderer) = difference_renderer {
                                        renderer.render_blended(
                                            device,
                                            queue,
                                            encoder,
                                            target_view,
                                            plot_rect,
                                            true,
                                            1.0,
                                        );
                                    }
                                }
                            }
                            match to {
                                rawscope_render::ScatterDensityMode::AbsoluteDensity => {
                                    scatter_density_renderer.render_blended(
                                        encoder,
                                        target_view,
                                        plot_rect,
                                        false,
                                        point_frame.apply_density(transition_alpha),
                                    );
                                }
                                rawscope_render::ScatterDensityMode::FilteredDifference => {
                                    if let Some(renderer) = difference_renderer {
                                        renderer.render_blended(
                                            device,
                                            queue,
                                            encoder,
                                            target_view,
                                            plot_rect,
                                            false,
                                            transition_alpha,
                                        );
                                    }
                                }
                            }
                        } else if density_mode
                            == rawscope_render::ScatterDensityMode::FilteredDifference
                        {
                            if let Some(renderer) = difference_renderer {
                                renderer.render(device, queue, encoder, target_view, plot_rect);
                            }
                        } else {
                            scatter_density_renderer.render_blended(
                                encoder,
                                target_view,
                                plot_rect,
                                true,
                                point_frame.apply_density(1.0),
                            );
                        }
                        if density_mode == rawscope_render::ScatterDensityMode::AbsoluteDensity {
                            if let Some(point_reveal_renderer) = point_reveal_renderer {
                                point_reveal_renderer.render_with_frame(
                                    queue,
                                    encoder,
                                    target_view,
                                    plot_rect,
                                    transition_point_frame,
                                );
                            }
                        }
                        scatter_brush_overlay_renderer.render(
                            queue,
                            encoder,
                            target_view,
                            brush_screen_rect,
                            plot_rect,
                        );
                        if let Some(renderer) = self.scatter_inspection_overlay_renderer.as_ref() {
                            renderer.render(
                                queue,
                                encoder,
                                target_view,
                                plot_rect,
                                inspection_overlay_inputs.pinned,
                                inspection_overlay_inputs.hovered,
                            );
                        }

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
                (Some(plot_rect), DemoMode::Timeline) => {
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
                            selection.project_to_screen(viewport, plot_rect.screen_size())
                        });

                    gpu.render_frame(|device, queue, target_view, encoder| {
                        timeline_density_renderer.render(encoder, target_view, plot_rect);
                        scatter_brush_overlay_renderer.render(
                            queue,
                            encoder,
                            target_view,
                            brush_screen_rect,
                            plot_rect,
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
                (None, _) => gpu.render_frame(|device, queue, target_view, encoder| {
                    let clear_pass =
                        encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                            label: Some("RawScope non-plot surface clear pass"),
                            color_attachments: &[Some(
                                egui_wgpu::wgpu::RenderPassColorAttachment {
                                    view: target_view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: egui_wgpu::wgpu::Operations {
                                        load: egui_wgpu::wgpu::LoadOp::Clear(
                                            egui_wgpu::wgpu::Color {
                                                r: 0.012,
                                                g: 0.015,
                                                b: 0.025,
                                                a: 1.0,
                                            },
                                        ),
                                        store: egui_wgpu::wgpu::StoreOp::Store,
                                    },
                                },
                            )],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                    drop(clear_pass);

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
                            label: Some("RawScope egui non-plot surface pass"),
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
                }),
            }
        };

        if let Some(egui_renderer) = self.egui_renderer.as_mut() {
            for texture_id in &textures_delta.free {
                egui_renderer.free_texture(texture_id);
            }
        }

        match render_status {
            Ok(ClearFrameStatus::Presented) => {
                if !self.render_schedule.is_refining() {
                    self.update_window_title();
                }
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
        if transition_frame.running {
            self.request_redraw();
        }
    }
}

impl WorkbenchApp {
    fn prepare_scheduled_timeline_density(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(work) = self.timeline_render_schedule.next_work() else {
            return Ok(());
        };
        if let Err(error) = self.recompute_timeline_density() {
            self.timeline_render_schedule.work_failed();
            return Err(error);
        }
        if self.timeline_render_schedule.work_completed(work) {
            self.begin_visual_transition(rawscope_render::TransitionKind::DensityRefresh);
        }
        Ok(())
    }
}
