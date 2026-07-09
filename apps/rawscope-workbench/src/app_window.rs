//! Window bootstrap and redraw helpers for the workbench shell.

use std::error::Error;

use rawscope_render::BrushScreenSize;
use tracing::info;
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, window::Window};

use crate::{
    app::{WorkbenchApp, INITIAL_HEIGHT, INITIAL_WIDTH, WINDOW_TITLE},
    demo::DemoMode,
};

impl WorkbenchApp {
    pub(crate) fn create_window_and_gpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        if self.window.is_some() {
            return Ok(());
        }

        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));
        let window = std::sync::Arc::new(event_loop.create_window(attributes)?);
        let gpu = pollster::block_on(rawscope_gpu::GpuContext::new(window.clone()))?;
        let adapter_info = gpu.adapter_info();
        info!(
            adapter = %adapter_info.adapter_name,
            backend = %adapter_info.backend,
            device_type = %adapter_info.device_type,
            surface_format = %adapter_info.surface_format,
            present_mode = %adapter_info.present_mode,
            alpha_mode = %adapter_info.alpha_mode,
            "RawScope WGPU adapter selected"
        );

        match self.demo_mode {
            DemoMode::Scatter => self.prepare_scatter_demo(&gpu)?,
            DemoMode::Timeline => self.prepare_timeline_demo(&gpu)?,
        }

        window.request_redraw();
        self.window = Some(window);
        self.gpu = Some(gpu);
        self.initialize_ui_integration();
        self.update_window_title();

        Ok(())
    }

    pub(crate) fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn screen_size(&self) -> BrushScreenSize {
        self.window
            .as_ref()
            .map(|window| {
                let size = window.inner_size();
                BrushScreenSize::new(size.width as f32, size.height as f32)
            })
            .unwrap_or_else(|| BrushScreenSize::new(0.0, 0.0))
    }

    pub(crate) fn update_window_title(&self) {
        let Some(window) = &self.window else {
            return;
        };
        window.set_title(&self.ui_state().window_title());
    }

    pub(crate) fn cursor_fraction(&self) -> Option<(f32, f32)> {
        let window = self.window.as_ref()?;
        let cursor_position = self.cursor_position?;
        let window_size = window.inner_size();

        let window_has_area = window_size.width > 0 && window_size.height > 0;
        if !window_has_area {
            return None;
        }

        let x_fraction = (cursor_position.x / window_size.width as f64) as f32;
        let y_fraction = (cursor_position.y / window_size.height as f64) as f32;
        Some((x_fraction.clamp(0.0, 1.0), y_fraction.clamp(0.0, 1.0)))
    }
}
