mod app;
mod app_brush;
mod app_events;
mod app_export;
mod app_render;
mod app_timeline;
mod app_timeline_brush;
mod demo;

use std::{error::Error, io};

use app::WorkbenchApp;
use demo::DemoMode;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let demo_mode = DemoMode::from_args(std::env::args().skip(1))
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let event_loop = EventLoop::new()?;
    let mut app = WorkbenchApp::new(demo_mode);
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .try_init();
}
