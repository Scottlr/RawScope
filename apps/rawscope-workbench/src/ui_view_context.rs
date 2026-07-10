//! View-axis and summary context projection for density views.

use egui::{vec2, Color32, Pos2, Rect, RichText, Sense, Ui};
use rawscope_data::{DatasetFieldRole, DatasetIdentity};
use rawscope_render::{
    scatter_axes_context_with_options, timeline_axes_context, AxisValueFormat, ScatterAxesOptions,
    ScatterMarginalSummary, ScatterReferenceGuide, ScatterReferenceGuideKind, SummaryBin,
    TimelineMarginalSummary, TimelineOverviewSummary,
};

use crate::{
    app::WorkbenchApp,
    demo::DemoMode,
    ui::{ActiveView, WorkbenchSurface, WorkbenchViewAxes},
};

const SUMMARY_STRIP_HEIGHT_PX: f32 = 14.0;
const SUMMARY_STRIP_MIN_WIDTH_PX: f32 = 160.0;
const SUMMARY_STRIP_GAP_PX: f32 = 1.0;
const MAX_AXIS_TICK_COUNT: usize = 6;
const MAX_TIME_AXIS_TICK_COUNT: usize = 6;
const MAX_LANE_LABEL_COUNT: usize = 7;

/// UI-ready projection for the current summary context.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkbenchViewContextUiState {
    pub(crate) active_view: ActiveView,
    pub(crate) scatter_marginals: Option<ScatterMarginalSummary>,
    pub(crate) timeline_marginals: Option<TimelineMarginalSummary>,
    pub(crate) timeline_overview: Option<TimelineOverviewSummary>,
}

/// Build an egui-ready view projection for the current primary surface.
pub(crate) fn view_axes_ui_state(app: &WorkbenchApp) -> Option<WorkbenchViewAxes> {
    if app.visible_surface != WorkbenchSurface::Primary {
        return None;
    }

    match app.demo_mode {
        DemoMode::Scatter => {
            let viewport = app.scatter.viewport?;
            let (x_label, y_label) = axis_field_labels(
                app.dataset_identity.as_ref(),
                (DatasetFieldRole::X, "x"),
                (DatasetFieldRole::Y, "y"),
            );

            let use_rating_axes =
                app.active_dataset_profile == Some(rawscope_data::DatasetProfileId::LichessGames);
            let value_format = if use_rating_axes {
                AxisValueFormat::Integer
            } else {
                AxisValueFormat::Decimal {
                    max_fraction_digits: 1,
                }
            };
            let guides = use_rating_axes
                .then(|| ScatterReferenceGuide {
                    kind: ScatterReferenceGuideKind::Equality,
                    label: "equal rating".to_string(),
                })
                .into_iter()
                .collect();

            Some(WorkbenchViewAxes::Scatter(
                scatter_axes_context_with_options(
                    viewport.x_range(),
                    viewport.y_range(),
                    x_label,
                    y_label,
                    ScatterAxesOptions {
                        target_tick_count: MAX_AXIS_TICK_COUNT,
                        x_format: value_format,
                        y_format: value_format,
                        guides,
                    },
                ),
            ))
        }
        DemoMode::Timeline => {
            let viewport = app.timeline.viewport?;

            let lane_labels = app
                .dataset_identity
                .as_ref()
                .map_or(&[][..], |identity| identity.lane_labels.as_slice());
            Some(WorkbenchViewAxes::Timeline(timeline_axes_context(
                viewport.time_range(),
                viewport.lane_count(),
                lane_labels,
                MAX_TIME_AXIS_TICK_COUNT,
                MAX_LANE_LABEL_COUNT,
            )))
        }
    }
}

/// Build an egui-ready summary projection for the current primary surface.
pub(crate) fn view_context_ui_state(app: &WorkbenchApp) -> Option<WorkbenchViewContextUiState> {
    if app.visible_surface != WorkbenchSurface::Primary {
        return None;
    }

    let active_view = ActiveView::from_demo_mode(app.demo_mode);

    match app.demo_mode {
        DemoMode::Scatter => {
            app.scatter
                .marginal_summary
                .as_ref()
                .cloned()
                .map(|scatter_marginals| WorkbenchViewContextUiState {
                    active_view,
                    scatter_marginals: Some(scatter_marginals),
                    timeline_marginals: None,
                    timeline_overview: None,
                })
        }
        DemoMode::Timeline => {
            let timeline_marginals = app.timeline.marginal_summary.as_ref().cloned();
            let timeline_overview = app.timeline.overview_summary.as_ref().cloned();
            if timeline_marginals.is_none() && timeline_overview.is_none() {
                return None;
            }

            Some(WorkbenchViewContextUiState {
                active_view,
                scatter_marginals: None,
                timeline_marginals,
                timeline_overview,
            })
        }
    }
}

/// Draw the compact summary context inside the right panel.
pub(crate) fn show_view_context(
    ui: &mut Ui,
    context: Option<&WorkbenchViewContextUiState>,
) -> bool {
    let Some(context) = context else {
        return false;
    };

    ui.heading("View Context");

    match context.active_view {
        ActiveView::Scatter => show_scatter_marginals(ui, context.scatter_marginals.as_ref()),
        ActiveView::Timeline => {
            show_timeline_marginals(ui, context.timeline_marginals.as_ref());
            ui.add_space(4.0);
            show_timeline_overview(ui, context.timeline_overview.as_ref());
        }
    }

    true
}

pub(crate) fn show_scatter_marginals(ui: &mut Ui, summary: Option<&ScatterMarginalSummary>) {
    let Some(summary) = summary else {
        ui.label("Scatter marginals unavailable.");
        return;
    };

    ui.label(RichText::new("Scatter marginals").strong());
    draw_summary_strip(
        ui,
        "x",
        &summary.x_bins,
        summary.max_x_count,
        Color32::from_rgb(88, 148, 214),
    );
    draw_summary_strip(
        ui,
        "y",
        &summary.y_bins,
        summary.max_y_count,
        Color32::from_rgb(98, 171, 126),
    );
}

fn show_timeline_marginals(ui: &mut Ui, summary: Option<&TimelineMarginalSummary>) {
    let Some(summary) = summary else {
        ui.label("Timeline marginals unavailable.");
        return;
    };

    ui.label(RichText::new("Timeline marginals").strong());
    draw_summary_strip(
        ui,
        "time",
        &summary.time_bins,
        summary.max_time_count,
        Color32::from_rgb(201, 151, 74),
    );
    draw_summary_strip(
        ui,
        "lane",
        &summary.lane_bins,
        summary.max_lane_count,
        Color32::from_rgb(150, 124, 208),
    );
}

pub(crate) fn show_timeline_overview(ui: &mut Ui, overview: Option<&TimelineOverviewSummary>) {
    let Some(overview) = overview else {
        ui.label("Timeline overview unavailable.");
        return;
    };

    ui.label(RichText::new("Timeline overview").strong());
    draw_summary_strip(
        ui,
        "full",
        &overview.time_bins,
        overview.max_time_count,
        Color32::from_rgb(119, 160, 194),
    );

    let window = overview.current_window();
    ui.label(format!(
        "current {}..{} of {}..{} ({:.0}%..{:.0}%)",
        overview.current_time_range.min,
        overview.current_time_range.max,
        overview.full_time_range.min,
        overview.full_time_range.max,
        window.start_fraction * 100.0,
        window.end_fraction * 100.0,
    ));
}

fn draw_summary_strip(
    ui: &mut Ui,
    label: &str,
    bins: &[SummaryBin],
    max_count: u32,
    fill_color: Color32,
) {
    if bins.is_empty() {
        ui.label(format!("{label}: unavailable"));
        return;
    }

    ui.horizontal(|ui| {
        ui.monospace(label);
        let width = ui.available_width().max(SUMMARY_STRIP_MIN_WIDTH_PX);
        let (rect, _response) =
            ui.allocate_exact_size(vec2(width, SUMMARY_STRIP_HEIGHT_PX), Sense::hover());
        let painter = ui.painter_at(rect);
        let bin_width = rect.width() / bins.len() as f32;

        for (bin_index, bin) in bins.iter().enumerate() {
            let bin_fraction = if max_count == 0 {
                0.0
            } else {
                bin.count as f32 / max_count as f32
            };
            let bar_height = rect.height() * bin_fraction;
            let left = rect.left() + (bin_index as f32 * bin_width);
            let right = if bin_index + 1 == bins.len() {
                rect.right()
            } else {
                left + bin_width - SUMMARY_STRIP_GAP_PX
            };
            let bar_rect = Rect::from_min_max(
                Pos2::new(left, rect.bottom() - bar_height),
                Pos2::new(right, rect.bottom()),
            );
            painter.rect_filled(
                bar_rect,
                0.0,
                fill_color.linear_multiply(0.65 + (0.35 * bin_fraction)),
            );
        }
    });
}

fn axis_field_labels(
    dataset_identity: Option<&DatasetIdentity>,
    x_axis: (DatasetFieldRole, &str),
    y_axis: (DatasetFieldRole, &str),
) -> (String, String) {
    let x_label = axis_field_label(dataset_identity, x_axis.0, x_axis.1);
    let y_label = axis_field_label(dataset_identity, y_axis.0, y_axis.1);

    (x_label, y_label)
}

fn axis_field_label(
    dataset_identity: Option<&DatasetIdentity>,
    role: DatasetFieldRole,
    fallback: &str,
) -> String {
    dataset_identity
        .and_then(|identity| {
            identity
                .field_bindings
                .iter()
                .find(|binding| binding.role == role)
                .map(|binding| binding.column_name.clone())
        })
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, U64Range};
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
        let app = WorkbenchApp {
            visible_surface: WorkbenchSurface::Primary,
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

        let context = view_context_ui_state(&app).expect("scatter context should project");

        assert_eq!(context.active_view, ActiveView::Scatter);
        assert!(context.scatter_marginals.is_some());
        assert!(context.timeline_marginals.is_none());
        assert!(context.timeline_overview.is_none());
    }

    #[test]
    fn timeline_context_projection_uses_timeline_summaries() {
        let (timeline_marginals, timeline_overview) = timeline_context();
        let app = WorkbenchApp {
            visible_surface: WorkbenchSurface::Primary,
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

        let context = view_context_ui_state(&app).expect("timeline context should project");

        assert_eq!(context.active_view, ActiveView::Timeline);
        assert!(context.scatter_marginals.is_none());
        assert_eq!(context.timeline_marginals, Some(timeline_marginals));
        assert_eq!(context.timeline_overview, Some(timeline_overview));
    }

    #[test]
    fn hidden_surface_does_not_project_view_context() {
        let app = WorkbenchApp {
            visible_surface: WorkbenchSurface::Missingness,
            ..WorkbenchApp::default()
        };

        assert!(view_context_ui_state(&app).is_none());
    }

    #[test]
    fn lichess_axes_use_integer_ticks_and_equality_guide() {
        let app = WorkbenchApp {
            visible_surface: WorkbenchSurface::Primary,
            demo_mode: DemoMode::Scatter,
            active_dataset_profile: Some(rawscope_data::DatasetProfileId::LichessGames),
            scatter: crate::app::ScatterWorkbenchState {
                viewport: Some(rawscope_render::ScatterViewport::new(
                    F32Range::new(1_000.0, 2_000.0),
                    F32Range::new(1_000.0, 2_000.0),
                )),
                ..crate::app::ScatterWorkbenchState::default()
            },
            ..WorkbenchApp::default()
        };

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
}
