use std::{error::Error, sync::Arc};

use rawscope_gpu::{ClearFrameStatus, GpuContext, DEFAULT_CLEAR_COLOR};
use tracing::{error, info, warn};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_TITLE: &str = "RawScope Workbench";
const INITIAL_WIDTH: f64 = 1280.0;
const INITIAL_HEIGHT: f64 = 720.0;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let event_loop = EventLoop::new()?;
    let mut app = WorkbenchApp::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .try_init();
}

#[derive(Default)]
struct WorkbenchApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
}

impl WorkbenchApp {
    fn create_window_and_gpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn Error>> {
        if self.window.is_some() {
            return Ok(());
        }

        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let gpu = pollster::block_on(GpuContext::new(window.clone()))?;

        let diagnostics = gpu.diagnostics();
        info!(
            adapter = %diagnostics.adapter_name,
            backend = %diagnostics.backend,
            device_type = %diagnostics.device_type,
            surface_format = %diagnostics.surface_format,
            present_mode = %diagnostics.present_mode,
            alpha_mode = %diagnostics.alpha_mode,
            "RawScope WGPU adapter selected"
        );
        info!(features = %diagnostics.adapter_features, "adapter features");
        info!(limits = %diagnostics.adapter_limits, "adapter limits");

        window.request_redraw();
        self.window = Some(window);
        self.gpu = Some(gpu);

        Ok(())
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };

        match gpu.clear_frame(DEFAULT_CLEAR_COLOR) {
            Ok(ClearFrameStatus::Presented) => {}
            Ok(ClearFrameStatus::SkippedZeroSizedSurface | ClearFrameStatus::SkippedOccluded) => {}
            Ok(ClearFrameStatus::SkippedTimeout | ClearFrameStatus::Reconfigured) => {
                self.request_redraw();
            }
            Err(err) => {
                error!(error = %err, "failed to render clear frame");
                event_loop.exit();
            }
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

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

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.request_redraw();
    }
}
