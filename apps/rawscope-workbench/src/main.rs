mod app;
mod app_brush;
mod app_events;
mod demo;

use std::error::Error;

use app::WorkbenchApp;
use winit::event_loop::EventLoop;

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
