use rawscope_core::{F32Range, U64Range};
use rawscope_data::{LoadedColumnKind, VisualFieldDescriptor, VisualFieldSummary};
use rawscope_render::{
    ScatterMarginalSummary, SummaryBin, TimelineMarginalSummary, TimelineOverviewSummary,
};

use super::{
    view_axes_ui_state, view_context_ui_state, ActiveView, WorkbenchSurface, WorkbenchViewAxes,
};
use crate::{app::WorkbenchApp, demo::DemoMode};

fn scatter_context() -> ScatterMarginalSummary {
    ScatterMarginalSummary {
        x_bins: vec![SummaryBin { index: 0, count: 2 }],
        y_bins: vec![SummaryBin { index: 0, count: 3 }],
        max_x_count: 2,
        max_y_count: 3,
    }
}

fn timeline_context() -> (TimelineMarginalSummary, TimelineOverviewSummary) {
    (
        TimelineMarginalSummary {
            time_bins: vec![SummaryBin { index: 0, count: 4 }],
            lane_bins: vec![SummaryBin { index: 0, count: 1 }],
            max_time_count: 4,
            max_lane_count: 1,
        },
        TimelineOverviewSummary {
            full_time_range: U64Range::new(100, 200),
            current_time_range: U64Range::new(120, 180),
            time_bins: vec![SummaryBin { index: 0, count: 4 }],
            max_time_count: 4,
        },
    )
}

#[test]
fn scatter_context_projection_uses_scatter_summary() {
    let mut app = WorkbenchApp {
        demo_mode: DemoMode::Scatter,
        scatter: crate::app::ScatterWorkbenchState {
            viewport: Some(rawscope_render::ScatterViewport::new(
                F32Range::new(0.0, 1.0),
                F32Range::new(0.0, 1.0),
            )),
            marginal_summary: Some(scatter_context()),
            ..crate::app::ScatterWorkbenchState::default()
        },
        ..WorkbenchApp::default()
    };
    app.workbench_state.visible_surface = WorkbenchSurface::Primary;

    let context = view_context_ui_state(&app).expect("scatter context should project");

    assert_eq!(context.active_view, ActiveView::Scatter);
    assert!(context.scatter_marginals.is_some());
    assert!(context.timeline_marginals.is_none());
    assert!(context.timeline_overview.is_none());
}

#[test]
fn timeline_context_projection_uses_timeline_summaries() {
    let (timeline_marginals, timeline_overview) = timeline_context();
    let mut app = WorkbenchApp {
        demo_mode: DemoMode::Timeline,
        timeline: crate::app::TimelineWorkbenchState {
            viewport: Some(rawscope_render::TimelineViewport::new(
                U64Range::new(100, 200),
                4,
            )),
            marginal_summary: Some(timeline_marginals.clone()),
            overview_summary: Some(timeline_overview.clone()),
            ..crate::app::TimelineWorkbenchState::default()
        },
        ..WorkbenchApp::default()
    };
    app.workbench_state.visible_surface = WorkbenchSurface::Primary;

    let context = view_context_ui_state(&app).expect("timeline context should project");

    assert_eq!(context.active_view, ActiveView::Timeline);
    assert!(context.scatter_marginals.is_none());
    assert_eq!(context.timeline_marginals, Some(timeline_marginals));
    assert_eq!(context.timeline_overview, Some(timeline_overview));
}

#[test]
fn hidden_surface_does_not_project_view_context() {
    let mut app = WorkbenchApp::default();
    app.workbench_state.visible_surface = WorkbenchSurface::Missingness;

    assert!(view_context_ui_state(&app).is_none());
}

#[test]
fn typed_integer_axes_use_integer_ticks_and_optional_equality_guide() {
    let mut app = WorkbenchApp {
        demo_mode: DemoMode::Scatter,
        scatter: crate::app::ScatterWorkbenchState {
            viewport: Some(rawscope_render::ScatterViewport::new(
                F32Range::new(1_000.0, 2_000.0),
                F32Range::new(1_000.0, 2_000.0),
            )),
            ..crate::app::ScatterWorkbenchState::default()
        },
        scatter_filters: crate::app_scatter_filter::ScatterFilterState {
            catalog: Some(rawscope_data::VisualFieldCatalog {
                row_count: 0,
                fields: vec![
                    VisualFieldDescriptor {
                        column_name: "white_rating".into(),
                        column_index: 0,
                        source_kind: LoadedColumnKind::Integer,
                        summary: VisualFieldSummary::Empty { missing_count: 0 },
                    },
                    VisualFieldDescriptor {
                        column_name: "black_rating".into(),
                        column_index: 1,
                        source_kind: LoadedColumnKind::Integer,
                        summary: VisualFieldSummary::Empty { missing_count: 0 },
                    },
                ],
            }),
            ..crate::app_scatter_filter::ScatterFilterState::default()
        },
        scatter_projection: crate::app_scatter_projection::ScatterProjectionState {
            labels: rawscope_data::ScatterProjectionLabels {
                x_label: "white_rating".into(),
                y_label: "black_rating".into(),
            },
            show_equality_guide: true,
            ..Default::default()
        },
        ..WorkbenchApp::default()
    };
    app.workbench_state.active_dataset_profile =
        Some(rawscope_data::DatasetProfileId::LichessGames);
    app.workbench_state.visible_surface = WorkbenchSurface::Primary;

    let WorkbenchViewAxes::Scatter(axes) =
        view_axes_ui_state(&app).expect("Lichess axes should project")
    else {
        panic!("expected scatter axes");
    };

    assert!(axes.x.ticks.iter().all(|tick| !tick.label.contains('.')));
    assert_eq!(axes.guides.len(), 1);
    assert_eq!(
        axes.guides[0].kind,
        rawscope_render::ScatterReferenceGuideKind::Equality
    );
}

#[test]
fn mean_difference_axes_use_zero_guide() {
    let mut app = WorkbenchApp {
        demo_mode: DemoMode::Scatter,
        scatter: crate::app::ScatterWorkbenchState {
            viewport: Some(rawscope_render::ScatterViewport::new(
                F32Range::new(800.0, 3_000.0),
                F32Range::new(-1_000.0, 1_000.0),
            )),
            ..crate::app::ScatterWorkbenchState::default()
        },
        scatter_projection: crate::app_scatter_projection::ScatterProjectionState {
            active: rawscope_data::ScatterProjection::MeanDifference,
            labels: rawscope_data::ScatterProjectionLabels {
                x_label: "mean(white_rating, black_rating)".into(),
                y_label: "white_rating - black_rating".into(),
            },
            available: true,
            ..Default::default()
        },
        ..WorkbenchApp::default()
    };
    app.workbench_state.active_dataset_profile =
        Some(rawscope_data::DatasetProfileId::LichessGames);
    app.workbench_state.visible_surface = WorkbenchSurface::Primary;

    let WorkbenchViewAxes::Scatter(axes) = view_axes_ui_state(&app).unwrap() else {
        panic!("expected scatter axes");
    };
    assert_eq!(axes.x.label, "mean(white_rating, black_rating)");
    assert_eq!(axes.y.label, "white_rating - black_rating");
    assert_eq!(axes.guides.len(), 1);
    assert_eq!(
        axes.guides[0].kind,
        rawscope_render::ScatterReferenceGuideKind::Horizontal { y: 0.0 }
    );
}
