mod app;
mod app_aggregate;
mod app_brush;
mod app_comparison;
mod app_dataset_diff;
mod app_dataset_profile;
mod app_events;
mod app_export;
mod app_missingness;
mod app_plot_interaction;
mod app_render;
mod app_report_bundle;
#[cfg(test)]
mod app_report_bundle_tests;
mod app_selection;
mod app_timeline;
mod app_timeline_brush;
mod app_visual_encoding;
mod app_window;
mod cli;
mod demo;
mod ui;
mod ui_comparison;
mod ui_controls;
mod ui_dataset_diff;
mod ui_drilldown;
mod ui_missingness;
mod ui_plot_axes;
mod ui_plot_surface;
mod ui_theme;
mod ui_view_context;
mod ui_visual_encoding;

use std::{error::Error, io};

use app::WorkbenchApp;
use cli::WorkbenchArgs;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let args = WorkbenchArgs::parse(std::env::args().skip(1))
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let event_loop = EventLoop::new()?;
    let mut app = WorkbenchApp::new(args);
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .try_init();
}
