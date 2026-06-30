//! Brush interaction helpers for the workbench scatter-density demo.

use rawscope_render::{BrushScreenPoint, BrushScreenSize, ScatterBrushDrag, SelectedRegionSummary};
use tracing::info;
use winit::dpi::PhysicalPosition;

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn begin_brush(&mut self) {
        self.last_drag_position = None;
        self.brush_drag_start = self.cursor_position;
        self.update_brush_from_cursor();
    }

    pub(crate) fn update_brush_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = Some(position);
        self.update_brush_from_cursor();
    }

    pub(crate) fn end_brush(&mut self) {
        self.finalize_brush_from_drag();
        self.log_selection_summary("finalized");
        self.brush_drag_start = None;
        self.active_brush_drag = None;
        self.update_window_title();
        self.request_redraw();
    }

    pub(crate) fn clear_brush(&mut self) {
        let had_selection = self.active_brush_selection.is_some()
            || self.active_brush_drag.is_some()
            || self.selection_summary.is_some();
        self.active_brush_drag = None;
        self.active_brush_selection = None;
        self.selection_summary = None;
        self.brush_drag_start = None;
        self.update_window_title();

        if had_selection {
            info!("RawScope scatter brush cleared");
        }
    }

    pub(crate) fn brush_is_active(&self) -> bool {
        self.brush_drag_start.is_some()
    }

    fn update_brush_from_cursor(&mut self) {
        let Some(brush_drag_start) = self.brush_drag_start else {
            return;
        };
        let Some(cursor_position) = self.cursor_position else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.viewport else {
            return;
        };

        let screen_size = BrushScreenSize::new(
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        let brush_start =
            BrushScreenPoint::new(brush_drag_start.x as f32, brush_drag_start.y as f32);
        let brush_end = BrushScreenPoint::new(cursor_position.x as f32, cursor_position.y as f32);
        let next_drag = ScatterBrushDrag::from_screen_points(brush_start, brush_end, screen_size);

        self.active_brush_drag = next_drag;
        self.active_brush_selection =
            next_drag.and_then(|drag| drag.finalize(screen_size, viewport));
        self.selection_summary = self
            .active_brush_selection
            .map(|selection| SelectedRegionSummary::from_points(&self.points, selection));
        self.update_window_title();
        self.request_redraw();
    }

    fn finalize_brush_from_drag(&mut self) {
        let Some(active_drag) = self.active_brush_drag else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let Some(viewport) = self.viewport else {
            return;
        };

        let window_size = window.inner_size();
        let screen_size = BrushScreenSize::new(window_size.width as f32, window_size.height as f32);
        self.active_brush_selection = active_drag.finalize(screen_size, viewport);
        self.selection_summary = self
            .active_brush_selection
            .map(|selection| SelectedRegionSummary::from_points(&self.points, selection));
    }

    fn log_selection_summary(&self, reason: &'static str) {
        let Some(summary) = self.selection_summary else {
            return;
        };

        info!(
            reason,
            selected_row_count = summary.selected_row_count,
            total_row_count = summary.total_row_count,
            selected_percentage = summary.selected_percentage,
            brush_x_min = summary.brush_x_range.min,
            brush_x_max = summary.brush_x_range.max,
            brush_y_min = summary.brush_y_range.min,
            brush_y_max = summary.brush_y_range.max,
            cluster_count = summary.category_counts.cluster,
            background_count = summary.category_counts.background,
            outlier_count = summary.category_counts.outlier,
            top_category = ?summary.top_category,
            "RawScope scatter brush summary"
        );
    }
}
