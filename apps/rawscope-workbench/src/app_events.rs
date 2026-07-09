//! Winit event translation for RawScope density views.

use rawscope_gpu::ClearFrameStatus;
use tracing::{error, warn};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{app::WorkbenchApp, demo::PointCountPreset};

impl ApplicationHandler for WorkbenchApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
                self.cursor_position = Some(position);
                if event_consumed {
                    return;
                }
                if self.demo_mode.is_scatter() && self.brush_is_active() {
                    self.update_brush_to_cursor(position);
                } else if self.demo_mode.is_timeline() && self.timeline_brush_is_active() {
                    self.update_timeline_brush_to_cursor(position);
                } else if self.demo_mode.is_scatter() && self.last_drag_position.is_some() {
                    self.pan_to_cursor(position);
                } else if self.demo_mode.is_timeline() && self.last_drag_position.is_some() {
                    self.pan_timeline_to_cursor(position);
                }
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
            WindowEvent::MouseInput { state, button, .. } if !event_consumed => {
                match (state, button) {
                    (ElementState::Pressed, MouseButton::Right) if self.demo_mode.is_scatter() => {
                        self.begin_brush();
                    }
                    (ElementState::Pressed, MouseButton::Right) if self.demo_mode.is_timeline() => {
                        self.begin_timeline_brush();
                    }
                    (ElementState::Pressed, MouseButton::Left)
                        if self.demo_mode.is_scatter() && self.modifiers.shift_key() =>
                    {
                        self.begin_brush();
                    }
                    (ElementState::Pressed, MouseButton::Left)
                        if self.demo_mode.is_timeline() && self.modifiers.shift_key() =>
                    {
                        self.begin_timeline_brush();
                    }
                    (ElementState::Pressed, MouseButton::Left | MouseButton::Middle)
                        if self.demo_mode.is_scatter() =>
                    {
                        self.begin_pan();
                    }
                    (ElementState::Pressed, MouseButton::Left | MouseButton::Middle)
                        if self.demo_mode.is_timeline() && !self.modifiers.shift_key() =>
                    {
                        self.begin_timeline_pan();
                    }
                    (ElementState::Released, MouseButton::Right) if self.demo_mode.is_scatter() => {
                        self.end_brush();
                    }
                    (ElementState::Released, MouseButton::Right)
                        if self.demo_mode.is_timeline() =>
                    {
                        self.end_timeline_brush();
                    }
                    (ElementState::Released, MouseButton::Left)
                        if self.demo_mode.is_scatter() && self.brush_is_active() =>
                    {
                        self.end_brush();
                    }
                    (ElementState::Released, MouseButton::Left)
                        if self.demo_mode.is_timeline() && self.timeline_brush_is_active() =>
                    {
                        self.end_timeline_brush();
                    }
                    (ElementState::Released, MouseButton::Left | MouseButton::Middle)
                        if self.demo_mode.is_scatter() =>
                    {
                        self.end_pan();
                    }
                    (ElementState::Released, MouseButton::Left | MouseButton::Middle)
                        if self.demo_mode.is_timeline() =>
                    {
                        self.end_pan();
                    }
                    _ => {}
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } if !event_consumed => {
                self.handle_keyboard_input(event.state, event.physical_key);
            }
            WindowEvent::Resized(size) => {
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

        match physical_key {
            PhysicalKey::Code(KeyCode::Escape) if self.demo_mode.is_scatter() => self.clear_brush(),
            PhysicalKey::Code(KeyCode::Escape) if self.demo_mode.is_timeline() => {
                self.clear_timeline_brush();
            }
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
