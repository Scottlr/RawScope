//! Window bootstrap and redraw helpers for the workbench shell.

use std::error::Error;

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

    pub(crate) fn update_window_title(&self) {
        let Some(window) = &self.window else {
            return;
        };
        window.set_title(&self.ui_state().window_title());
    }
}
