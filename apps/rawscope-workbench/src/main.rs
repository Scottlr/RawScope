#[allow(dead_code)]
mod active_generation;
mod app;
mod app_aggregate;
mod app_brush;
mod app_comparison;
mod app_dataset_diff;
mod app_dataset_profile;
mod app_events;
mod app_export;
mod app_export_v5;
mod app_inspection_presentation;
mod app_interaction_mode;
mod app_missingness;
mod app_plot_interaction;
mod app_render;
mod app_render_schedule;
mod app_report_bundle;
#[cfg(test)]
mod app_report_bundle_tests;
mod app_report_bundle_v4;
mod app_report_bundle_v5;
mod app_scatter_density;
mod app_scatter_difference;
mod app_scatter_filter;
mod app_scatter_inspection;
mod app_scatter_overlays;
mod app_scatter_point_reveal;
mod app_scatter_projection;
mod app_selection;
mod app_session;
mod app_session_visual_field;
mod app_timeline;
mod app_timeline_brush;
mod app_visual_encoding;
mod app_visual_transition;
mod app_window;
mod cli;
mod controllers;
#[allow(dead_code)]
mod degraded_state;
mod demo;
mod gpu_startup_job;
mod job_coordinator;
#[allow(dead_code)]
mod operation_error;
#[allow(dead_code)]
mod render_coordinator;
#[allow(dead_code)]
mod startup_job;
#[expect(
    dead_code,
    reason = "staged startup lifecycle foundation is integrated by a follow-up task"
)]
mod startup_lifecycle;
mod timeline_render_schedule;
mod ui;
mod ui_activity_rail;
mod ui_comparison;
mod ui_controls;
mod ui_dataset_diff;
mod ui_dataset_identity;
mod ui_difference_density;
mod ui_drilldown;
mod ui_filters;
mod ui_inspection_tooltip;
mod ui_missingness;
mod ui_pinned_inspection;
mod ui_plot_axes;
mod ui_plot_surface;
#[allow(dead_code)]
mod ui_projection_cache;
mod ui_relief;
mod ui_scatter_inspection;
mod ui_shell;
mod ui_shell_layout;
mod ui_theme;
mod ui_view_context;
mod ui_visual_encoding;
mod ui_workspace_inspector;
#[allow(dead_code)]
mod workbench_event;
#[allow(dead_code)]
mod workbench_state;

use std::{error::Error, io};

use app::WorkbenchApp;
use cli::WorkbenchArgs;
use winit::event_loop::EventLoop;
use workbench_event::WorkbenchUserEvent;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let args = WorkbenchArgs::parse(std::env::args().skip(1))
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let event_loop = EventLoop::<WorkbenchUserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = WorkbenchApp::new_from_args(args, proxy).map_err(|error| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to submit startup resolution job: {error:?}"),
        )
    })?;
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .try_init();
}
