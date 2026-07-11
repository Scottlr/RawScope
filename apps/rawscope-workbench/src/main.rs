mod app;
mod app_aggregate;
mod app_brush;
mod app_comparison;
mod app_dataset_diff;
mod app_dataset_profile;
mod app_events;
mod app_export;
mod app_interaction_mode;
mod app_missingness;
mod app_plot_interaction;
mod app_render;
mod app_render_schedule;
mod app_report_bundle;
#[cfg(test)]
mod app_report_bundle_tests;
mod app_report_bundle_v4;
mod app_scatter_density;
mod app_scatter_difference;
mod app_scatter_filter;
mod app_scatter_inspection;
mod app_scatter_point_reveal;
mod app_scatter_projection;
mod app_selection;
mod app_session;
mod app_timeline;
mod app_timeline_brush;
mod app_visual_encoding;
mod app_visual_transition;
mod app_window;
mod cli;
mod demo;
mod ui;
mod ui_comparison;
mod ui_controls;
mod ui_dataset_diff;
mod ui_dataset_identity;
mod ui_difference_density;
mod ui_drilldown;
mod ui_filters;
mod ui_missingness;
mod ui_plot_axes;
mod ui_plot_surface;
mod ui_relief;
mod ui_scatter_inspection;
mod ui_shell;
mod ui_theme;
mod ui_view_context;
mod ui_visual_encoding;

use std::{error::Error, io};

use app::WorkbenchApp;
use app_session::resolve_workbench_startup;
use cli::WorkbenchArgs;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let args = WorkbenchArgs::parse(std::env::args().skip(1))
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let startup = resolve_workbench_startup(args)?;
    let event_loop = EventLoop::new()?;
    let mut app = WorkbenchApp::new(startup);
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .try_init();
}
