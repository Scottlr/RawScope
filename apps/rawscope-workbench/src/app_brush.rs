//! Brush interaction helpers for the workbench scatter-density demo.

use rawscope_render::{
    BrushScreenPoint, ScatterBrushDrag, ScatterSelectionEvidence, SelectedRegionSummary,
    SelectionEvidenceConfig,
};
use tracing::info;
use winit::dpi::PhysicalPosition;

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn begin_brush(&mut self) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        self.last_drag_position = None;
        self.scatter.brush_drag_start = self.cursor_position;
        self.update_brush_from_cursor();
    }

    pub(crate) fn update_brush_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        self.cursor_position = Some(position);
        self.update_brush_from_cursor();
    }

    pub(crate) fn end_brush(&mut self) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        self.finalize_brush_from_drag();
        self.build_selection_evidence();
        self.log_selection_summary("finalized");
        self.log_selection_evidence();
        self.scatter.brush_drag_start = None;
        self.scatter.active_brush_drag = None;
        self.update_window_title();
        self.request_redraw();
    }

    pub(crate) fn clear_brush(&mut self) {
        let had_selection = self.scatter.active_brush_selection.is_some()
            || self.scatter.active_brush_drag.is_some()
            || self.scatter.selection_summary.is_some()
            || self.scatter.selection_evidence.is_some();
        self.scatter.active_brush_drag = None;
        self.scatter.active_brush_selection = None;
        self.scatter.selection_summary = None;
        self.scatter.selection_evidence = None;
        self.scatter.brush_drag_start = None;
        self.update_window_title();

        if had_selection {
            info!("RawScope scatter brush cleared");
        }
    }

    pub(crate) fn brush_is_active(&self) -> bool {
        self.scatter.brush_drag_start.is_some()
    }

    fn update_brush_from_cursor(&mut self) {
        let Some(brush_drag_start) = self.scatter.brush_drag_start else {
            return;
        };
        let Some(cursor_position) = self.cursor_position else {
            return;
        };
        let Some(viewport) = self.scatter.viewport else {
            return;
        };

        let screen_size = self.screen_size();
        let brush_start =
            BrushScreenPoint::new(brush_drag_start.x as f32, brush_drag_start.y as f32);
        let brush_end = BrushScreenPoint::new(cursor_position.x as f32, cursor_position.y as f32);
        let next_drag = ScatterBrushDrag::from_screen_points(brush_start, brush_end, screen_size);

        self.scatter.active_brush_drag = next_drag;
        self.scatter.active_brush_selection =
            next_drag.and_then(|drag| drag.finalize(screen_size, viewport));
        self.scatter.selection_summary = self
            .scatter
            .active_brush_selection
            .map(|selection| SelectedRegionSummary::from_points(&self.scatter.points, selection));
        self.scatter.selection_evidence = None;
        self.update_window_title();
        self.request_redraw();
    }

    fn finalize_brush_from_drag(&mut self) {
        let Some(active_drag) = self.scatter.active_brush_drag else {
            return;
        };
        let Some(viewport) = self.scatter.viewport else {
            return;
        };

        let screen_size = self.screen_size();
        self.scatter.active_brush_selection = active_drag.finalize(screen_size, viewport);
        self.scatter.selection_summary = self
            .scatter
            .active_brush_selection
            .map(|selection| SelectedRegionSummary::from_points(&self.scatter.points, selection));
    }

    fn build_selection_evidence(&mut self) {
        let Some(selection) = self.scatter.active_brush_selection else {
            self.scatter.selection_evidence = None;
            return;
        };
        let Some(dataset_metadata) = self.dataset_metadata.clone() else {
            self.scatter.selection_evidence = None;
            return;
        };

        self.scatter.selection_evidence = Some(ScatterSelectionEvidence::from_points(
            &self.scatter.points,
            selection,
            dataset_metadata,
            self.scatter_evidence_row_count(),
            SelectionEvidenceConfig::default(),
        ));
    }

    fn scatter_evidence_row_count(&self) -> usize {
        if self.input.is_some() {
            return self.scatter.points.len();
        }

        self.scatter.active_preset.row_count
    }

    fn log_selection_summary(&self, reason: &'static str) {
        let Some(summary) = self.scatter.selection_summary else {
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
            unclassified_count = summary.category_counts.unclassified,
            top_category = ?summary.top_category,
            "RawScope scatter brush summary"
        );
    }

    fn log_selection_evidence(&self) {
        let Some(evidence) = &self.scatter.selection_evidence else {
            return;
        };

        info!(
            selected_row_count = evidence.selected_row_count,
            total_row_count = evidence.dataset_metadata.row_count,
            selected_percentage = evidence.selected_percentage,
            seed = evidence.dataset_metadata.seed,
            point_preset_row_count = evidence.point_preset_row_count,
            brush_x_min = evidence.brush_x_range.min,
            brush_x_max = evidence.brush_x_range.max,
            brush_y_min = evidence.brush_y_range.min,
            brush_y_max = evidence.brush_y_range.max,
            selected_x_range = ?evidence.selected_x_range,
            selected_y_range = ?evidence.selected_y_range,
            category_counts = ?evidence.category_counts,
            top_category = ?evidence.top_category,
            row_id_sample = ?evidence.selected_row_id_sample,
            record_sample = ?evidence.selected_record_sample,
            "RawScope scatter selection evidence"
        );
    }
}
