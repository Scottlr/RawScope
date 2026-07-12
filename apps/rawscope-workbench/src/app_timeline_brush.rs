//! Timeline brush interaction helpers for the workbench timeline-density demo.

use rawscope_evidence::TimelineEvidenceConfig;
use rawscope_render::{
    timeline_selection_drilldown, timeline_selection_drilldown_snapshot,
    timeline_selection_evidence_from_events, BrushScreenRect, TimelineBrushDrag,
    TimelineSelectionSummary,
};
use tracing::info;
use winit::dpi::PhysicalPosition;

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn begin_timeline_brush(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }
        let Some(brush_start) = self.plot_local_cursor_point() else {
            return;
        };

        self.last_drag_position = None;
        self.timeline.brush_drag_start = Some(brush_start);
        self.update_timeline_brush_from_cursor();
    }

    pub(crate) fn update_timeline_brush_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        self.cursor_position = Some(position);
        self.update_timeline_brush_from_cursor();
    }

    pub(crate) fn end_timeline_brush(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        self.finalize_timeline_brush_from_drag();
        self.publish_timeline_active_selection();
        if let (Some(selection), Some(snapshot)) = (
            self.timeline.active_brush_selection,
            self.workbench_state
                .active_selection
                .as_ref()
                .and_then(|active| active.analysis_snapshot.as_ref()),
        ) {
            self.timeline.selection_summary = Some(TimelineSelectionSummary::from_snapshot(
                &self.timeline.events,
                snapshot,
                selection,
                self.timeline
                    .viewport
                    .map_or(selection.lane_range.end_exclusive, |viewport| {
                        viewport.lane_count()
                    }),
            ));
        }
        self.build_timeline_selection_evidence();
        self.build_timeline_selection_drilldown();
        self.rebuild_active_comparison();
        self.log_timeline_selection_summary("finalized");
        self.log_timeline_selection_evidence();
        self.log_timeline_selection_drilldown();
        self.timeline.brush_drag_start = None;
        self.timeline.active_brush_drag = None;
        self.update_window_title();
        self.request_redraw();
    }

    pub(crate) fn clear_timeline_brush(&mut self) {
        let had_selection = self.timeline_brush_is_active()
            || self.timeline.active_brush_selection.is_some()
            || self.timeline.selection_summary.is_some()
            || self.timeline.selection_evidence.is_some()
            || self.timeline.selection_drilldown.is_some();
        self.timeline.brush_drag_start = None;
        self.timeline.active_brush_drag = None;
        self.timeline.active_brush_selection = None;
        self.timeline.selection_summary = None;
        self.timeline.selection_evidence = None;
        self.timeline.selection_drilldown = None;
        self.clear_active_selection();
        self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
        self.update_window_title();
        self.request_redraw();

        if had_selection {
            info!("RawScope timeline brush cleared");
        }
    }

    pub(crate) fn cancel_timeline_brush_gesture(&mut self) {
        self.timeline.brush_drag_start = None;
        self.timeline.active_brush_drag = None;
        self.request_redraw();
    }

    pub(crate) fn timeline_brush_is_active(&self) -> bool {
        self.timeline.brush_drag_start.is_some()
    }

    fn update_timeline_brush_from_cursor(&mut self) {
        let Some(brush_drag_start) = self.timeline.brush_drag_start else {
            return;
        };
        let Some(cursor_position) = self.cursor_position else {
            return;
        };
        let Some(viewport) = self.timeline.viewport else {
            return;
        };

        let Some(screen_size) = self.plot_screen_size() else {
            return;
        };
        let Some(brush_end) = self.clamped_plot_local_point(cursor_position) else {
            return;
        };
        let brush_start = brush_drag_start;
        let next_drag = BrushScreenRect::from_points(brush_start, brush_end, screen_size)
            .map(TimelineBrushDrag::from_screen_rect);

        self.timeline.active_brush_drag = next_drag;
        self.timeline.active_brush_selection =
            next_drag.and_then(|drag| drag.finalize(screen_size, viewport));
        self.timeline.selection_summary = self.timeline.active_brush_selection.map(|selection| {
            TimelineSelectionSummary::from_events(
                &self.timeline.events,
                selection,
                viewport.lane_count(),
            )
        });
        self.timeline.selection_evidence = None;
        self.timeline.selection_drilldown = None;
        self.update_window_title();
        self.request_redraw();
    }

    fn finalize_timeline_brush_from_drag(&mut self) {
        let Some(active_drag) = self.timeline.active_brush_drag else {
            return;
        };
        let Some(viewport) = self.timeline.viewport else {
            return;
        };

        let Some(screen_size) = self.plot_screen_size() else {
            return;
        };
        self.timeline.active_brush_selection = active_drag.finalize(screen_size, viewport);
        self.timeline.selection_summary = self.timeline.active_brush_selection.map(|selection| {
            TimelineSelectionSummary::from_events(
                &self.timeline.events,
                selection,
                viewport.lane_count(),
            )
        });
    }

    fn build_timeline_selection_evidence(&mut self) {
        let Some(selection) = self.timeline.active_brush_selection else {
            self.timeline.selection_evidence = None;
            return;
        };
        let Some(viewport) = self.timeline.viewport else {
            self.timeline.selection_evidence = None;
            return;
        };
        let Some(dataset_metadata) = self.workbench_state.dataset_metadata.clone() else {
            self.timeline.selection_evidence = None;
            return;
        };

        self.timeline.selection_evidence = Some(timeline_selection_evidence_from_events(
            &self.timeline.events,
            selection,
            viewport.lane_count(),
            dataset_metadata,
            self.timeline.events.len(),
            TimelineEvidenceConfig::default(),
        ));
    }

    fn build_timeline_selection_drilldown(&mut self) {
        let Some(selection) = self.timeline.active_brush_selection else {
            self.timeline.selection_drilldown = None;
            return;
        };

        let snapshot = self
            .workbench_state
            .active_selection
            .as_ref()
            .and_then(|active| active.analysis_snapshot.as_ref())
            .cloned();
        self.timeline.selection_drilldown = snapshot
            .as_ref()
            .map(|snapshot| {
                timeline_selection_drilldown_snapshot(
                    &self.timeline.events,
                    snapshot,
                    self.timeline.source_rows.as_ref(),
                    rawscope_render::DrilldownConfig::default(),
                )
            })
            .or_else(|| {
                Some(timeline_selection_drilldown(
                    &self.timeline.events,
                    selection,
                    self.timeline.source_rows.as_ref(),
                    rawscope_render::DrilldownConfig::default(),
                ))
            });
    }

    fn log_timeline_selection_summary(&self, reason: &'static str) {
        let Some(summary) = &self.timeline.selection_summary else {
            return;
        };

        info!(
            reason,
            selected_event_count = summary.selected_event_count,
            total_event_count = summary.total_event_count,
            selected_percentage = summary.selected_percentage,
            brush_time_min = summary.selected_time_range.min,
            brush_time_max = summary.selected_time_range.max,
            lane_start = summary.selected_lane_range.start,
            lane_end_exclusive = summary.selected_lane_range.end_exclusive,
            lane_counts = ?summary.lane_counts,
            event_type_counts = ?summary.event_type_counts,
            top_lane = ?summary.top_lane,
            top_event_type = ?summary.top_event_type,
            selected_timestamp_range = ?summary.selected_timestamp_range,
            selected_value_range = ?summary.selected_value_range,
            "RawScope timeline brush summary"
        );
    }

    fn log_timeline_selection_evidence(&self) {
        let Some(evidence) = &self.timeline.selection_evidence else {
            return;
        };

        info!(
            selected_event_count = evidence.selected_event_count,
            total_event_count = evidence.event_count,
            selected_percentage = evidence.selected_percentage,
            seed = evidence.dataset_metadata.seed,
            dataset_row_count = evidence.dataset_metadata.row_count,
            brush_time_min = evidence.selected_time_range.min,
            brush_time_max = evidence.selected_time_range.max,
            lane_start = evidence.selected_lane_range.start,
            lane_end_exclusive = evidence.selected_lane_range.end_exclusive,
            lane_counts = ?evidence.lane_counts,
            event_type_counts = ?evidence.event_type_counts,
            top_lane = ?evidence.top_lane,
            top_event_type = ?evidence.top_event_type,
            selected_timestamp_range = ?evidence.selected_timestamp_range,
            selected_value_range = ?evidence.selected_value_range,
            row_id_sample = ?evidence.selected_row_id_sample,
            event_sample = ?evidence.selected_event_sample,
            "RawScope timeline selection evidence"
        );
    }

    fn log_timeline_selection_drilldown(&self) {
        let Some(drilldown) = &self.timeline.selection_drilldown else {
            return;
        };

        info!(
            selected_row_count = drilldown.selected_row_count,
            displayed_row_count = drilldown.displayed_row_count,
            sampled = drilldown.rows_are_sampled,
            column_count = drilldown.columns.len(),
            "RawScope timeline selection drilldown prepared"
        );
    }
}

#[expect(
    clippy::field_reassign_with_default,
    reason = "fixtures intentionally mutate scenario-specific fields"
)]
#[cfg(test)]
mod tests {
    use rawscope_core::{RowId, U64Range};
    use rawscope_data::{
        generate_synthetic_events, SyntheticEventConfig, TimelineEventKind, TimelineEventRecord,
    };
    use rawscope_evidence::TimelineLaneRange;
    use rawscope_render::{SelectionDrilldown, TimelineBrushSelection};

    use super::*;
    use crate::demo::DemoMode;

    #[test]
    fn build_selection_drilldown_populates_timeline_fallback_rows() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Timeline;
        app.timeline.events = vec![
            TimelineEventRecord {
                row_id: RowId(4),
                timestamp: 140,
                lane: 1,
                value: 2.5,
                kind: TimelineEventKind::Unclassified,
            },
            TimelineEventRecord {
                row_id: RowId(1),
                timestamp: 120,
                lane: 0,
                value: 1.5,
                kind: TimelineEventKind::Unclassified,
            },
        ];
        app.timeline.active_brush_selection = Some(TimelineBrushSelection {
            time_range: U64Range::new(100, 200),
            lane_range: TimelineLaneRange::new(0, 2),
        });

        app.build_timeline_selection_drilldown();

        let drilldown = app
            .timeline
            .selection_drilldown
            .expect("drilldown should exist");
        assert_eq!(drilldown.selected_row_count, 2);
        assert_eq!(drilldown.displayed_row_count, 2);
        assert_eq!(drilldown.columns[1].name, "timestamp");
        assert_eq!(drilldown.rows[0].row_id, RowId(1));
        assert_eq!(drilldown.rows[0].values[4], "unclassified");
    }

    #[test]
    fn clear_timeline_brush_clears_timeline_drilldown() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Timeline;
        app.workbench_state.dataset_identity =
            Some(generate_synthetic_events(SyntheticEventConfig::new(42, 1)).identity);
        app.timeline.events = vec![TimelineEventRecord {
            row_id: RowId(1),
            timestamp: 120,
            lane: 0,
            value: 1.5,
            kind: TimelineEventKind::Unclassified,
        }];
        app.timeline.active_brush_selection = Some(TimelineBrushSelection {
            time_range: U64Range::new(100, 200),
            lane_range: TimelineLaneRange::new(0, 1),
        });
        app.publish_timeline_active_selection();
        app.timeline.selection_drilldown = Some(SelectionDrilldown {
            selected_row_count: 1,
            displayed_row_count: 1,
            rows_are_sampled: false,
            columns: vec![],
            rows: vec![],
            completeness: rawscope_analysis::drilldown::DrilldownCompleteness::new(
                1,
                1,
                1,
                false,
                Vec::new(),
            ),
        });

        app.clear_timeline_brush();

        assert!(app.workbench_state.active_selection.is_none());
        assert!(app.timeline.selection_drilldown.is_none());
    }
}
