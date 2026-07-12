//! Winit event translation for RawScope density views.

use rawscope_gpu::ClearFrameStatus;
use tracing::{error, warn};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{
    app::WorkbenchApp, app_interaction_mode::interaction_mode_for_shortcut, demo::PointCountPreset,
    workbench_event::WorkbenchUserEvent,
};

impl ApplicationHandler<WorkbenchUserEvent> for WorkbenchApp {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WorkbenchUserEvent) {
        match event {
            WorkbenchUserEvent::JobCompleted { job_id, .. } => {
                match self.complete_startup_job(job_id) {
                    Ok(true) => {
                        if let Err(error) = self.create_window_and_gpu(event_loop) {
                            error!(error = %error, "failed to initialize RawScope workbench after startup resolution");
                            event_loop.exit();
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        error!(error = %error, "failed to resolve RawScope startup session");
                        event_loop.exit();
                    }
                }
                self.request_redraw();
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.startup_job.is_some() {
            return;
        }
        if let Err(err) = self.create_window_and_gpu(event_loop) {
            error!(error = %err, "failed to initialize RawScope workbench");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        let egui_response = self
            .egui_state
            .as_mut()
            .map(|state| state.on_window_event(window, &event));
        if let Some(response) = egui_response {
            if response.repaint {
                self.request_redraw();
            }
        }
        let event_consumed = egui_response.is_some_and(|response| response.consumed);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.update_pointer_position(position, !event_consumed);
            }
            WindowEvent::CursorLeft { .. } if self.active_pointer_gesture.is_none() => {
                self.cursor_position = None;
                self.inspect_cursor_position = None;
                self.clear_scatter_inspection_hover();
                self.update_pointer_cursor();
            }
            WindowEvent::MouseWheel { delta, .. }
                if !event_consumed && self.demo_mode.is_scatter() =>
            {
                self.zoom_at_cursor(delta);
            }
            WindowEvent::MouseWheel { delta, .. }
                if !event_consumed && self.demo_mode.is_timeline() =>
            {
                self.zoom_timeline_at_cursor(delta);
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => self.end_pointer_gesture(button),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } if !event_consumed => self.begin_pointer_gesture(button),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::Focused(false) if self.active_pointer_gesture.is_some() => {
                self.cancel_active_pointer_gesture();
            }
            WindowEvent::KeyboardInput { event, .. }
                if !event_consumed && !self.egui_context.text_edit_focused() =>
            {
                self.handle_keyboard_input(event.state, event.physical_key);
            }
            WindowEvent::Resized(size) => {
                self.plot_surface = None;
                if let Some(gpu) = self.gpu.as_mut() {
                    match gpu.resize(size) {
                        ClearFrameStatus::Reconfigured => self.request_redraw(),
                        ClearFrameStatus::SkippedZeroSizedSurface => {}
                        status => warn!(?status, "unexpected resize status"),
                    }
                }
            }
            WindowEvent::RedrawRequested => self.render(event_loop),
            _ => {}
        }
    }
}

impl WorkbenchApp {
    fn handle_keyboard_input(&mut self, state: ElementState, physical_key: PhysicalKey) {
        let key_is_pressed = state == ElementState::Pressed;
        if !key_is_pressed {
            return;
        }

        if let PhysicalKey::Code(key) = physical_key {
            if let Some(mode) = interaction_mode_for_shortcut(key) {
                self.set_interaction_mode(mode);
                return;
            }
        }

        match physical_key {
            PhysicalKey::Code(KeyCode::Escape) => self.handle_escape(),
            PhysicalKey::Code(KeyCode::KeyE) if self.demo_mode.is_scatter() => {
                self.export_selection_evidence();
            }
            PhysicalKey::Code(KeyCode::KeyE) if self.demo_mode.is_timeline() => {
                self.export_timeline_selection_evidence();
            }
            PhysicalKey::Code(KeyCode::KeyR) if self.demo_mode.is_scatter() => {
                self.reset_viewport();
            }
            PhysicalKey::Code(KeyCode::KeyR) if self.demo_mode.is_timeline() => {
                self.reset_timeline_viewport();
            }
            PhysicalKey::Code(KeyCode::Digit1) if self.demo_mode.is_scatter() => {
                self.switch_to_digit_preset('1');
            }
            PhysicalKey::Code(KeyCode::Digit2) if self.demo_mode.is_scatter() => {
                self.switch_to_digit_preset('2');
            }
            PhysicalKey::Code(KeyCode::Digit3) if self.demo_mode.is_scatter() => {
                self.switch_to_digit_preset('3');
            }
            PhysicalKey::Code(KeyCode::Digit4) if self.demo_mode.is_scatter() => {
                self.switch_to_digit_preset('4');
            }
            _ => {}
        }
    }

    fn switch_to_digit_preset(&mut self, digit: char) {
        if let Some(preset) = PointCountPreset::from_digit_key(digit) {
            self.switch_point_preset(preset);
        }
    }
}
