//! Timeline brush interaction helpers for the workbench timeline-density demo.

use rawscope_render::{
    BrushScreenPoint, BrushScreenRect, BrushScreenSize, TimelineBrushDrag, TimelineEvidenceConfig,
    TimelineSelectionEvidence, TimelineSelectionSummary,
};
use tracing::info;
use winit::dpi::PhysicalPosition;

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn begin_timeline_brush(&mut self) {
        if !self.demo_mode.is_timeline() {
            return;
        }

        self.last_drag_position = None;
        self.timeline_brush_drag_start = self.cursor_position;
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
        self.build_timeline_selection_evidence();
        self.log_timeline_selection_summary("finalized");
        self.log_timeline_selection_evidence();
        self.timeline_brush_drag_start = None;
        self.active_timeline_brush_drag = None;
        self.update_window_title();
        self.request_redraw();
    }

    pub(crate) fn clear_timeline_brush(&mut self) {
        let had_selection = self.timeline_brush_is_active()
            || self.active_timeline_brush_selection.is_some()
            || self.timeline_selection_summary.is_some()
            || self.timeline_selection_evidence.is_some();
        self.timeline_brush_drag_start = None;
        self.active_timeline_brush_drag = None;
        self.active_timeline_brush_selection = None;
        self.timeline_selection_summary = None;
        self.timeline_selection_evidence = None;
        self.update_window_title();
        self.request_redraw();

        if had_selection {
            info!("RawScope timeline brush cleared");
        }
    }

    pub(crate) fn timeline_brush_is_active(&self) -> bool {
        self.timeline_brush_drag_start.is_some()
    }

    fn update_timeline_brush_from_cursor(&mut self) {
        let Some(brush_drag_start) = self.timeline_brush_drag_start else {
            return;
        };
        let Some(cursor_position) = self.cursor_position else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.timeline_viewport else {
            return;
        };

        let window_size = window.inner_size();
        let screen_size = BrushScreenSize::new(window_size.width as f32, window_size.height as f32);
        let brush_start =
            BrushScreenPoint::new(brush_drag_start.x as f32, brush_drag_start.y as f32);
        let brush_end = BrushScreenPoint::new(cursor_position.x as f32, cursor_position.y as f32);
        let next_drag = BrushScreenRect::from_points(brush_start, brush_end, screen_size)
            .map(TimelineBrushDrag::from_screen_rect);

        self.active_timeline_brush_drag = next_drag;
        self.active_timeline_brush_selection =
            next_drag.and_then(|drag| drag.finalize(screen_size, viewport));
        self.timeline_selection_summary = self.active_timeline_brush_selection.map(|selection| {
            TimelineSelectionSummary::from_events(&self.events, selection, viewport.lane_count())
        });
        self.timeline_selection_evidence = None;
        self.update_window_title();
        self.request_redraw();
    }

    fn finalize_timeline_brush_from_drag(&mut self) {
        let Some(active_drag) = self.active_timeline_brush_drag else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.timeline_viewport else {
            return;
        };

        let window_size = window.inner_size();
        let screen_size = BrushScreenSize::new(window_size.width as f32, window_size.height as f32);
        self.active_timeline_brush_selection = active_drag.finalize(screen_size, viewport);
        self.timeline_selection_summary = self.active_timeline_brush_selection.map(|selection| {
            TimelineSelectionSummary::from_events(&self.events, selection, viewport.lane_count())
        });
    }

    fn build_timeline_selection_evidence(&mut self) {
        let Some(selection) = self.active_timeline_brush_selection else {
            self.timeline_selection_evidence = None;
            return;
        };
        let Some(viewport) = self.timeline_viewport else {
            self.timeline_selection_evidence = None;
            return;
        };
        let Some(dataset_metadata) = self.dataset_metadata.clone() else {
            self.timeline_selection_evidence = None;
            return;
        };

        self.timeline_selection_evidence = Some(TimelineSelectionEvidence::from_events(
            &self.events,
            selection,
            viewport.lane_count(),
            dataset_metadata,
            self.events.len(),
            TimelineEvidenceConfig::default(),
        ));
    }

    fn log_timeline_selection_summary(&self, reason: &'static str) {
        let Some(summary) = &self.timeline_selection_summary else {
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
        let Some(evidence) = &self.timeline_selection_evidence else {
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
}
