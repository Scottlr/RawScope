//! Brush interaction helpers for the workbench scatter-density demo.

use rawscope_evidence::{
    ScatterSelectionEvidence, ScatterSelectionGeometry, SelectionEvidenceConfig,
};
use rawscope_render::{
    scatter_selection_drilldown, scatter_selection_drilldown_masked,
    scatter_selection_drilldown_snapshot, selected_region_summary_masked,
    selected_region_summary_snapshot, ScatterBrushDrag, ScatterBrushSelection,
    SelectedRegionSummary,
};
use tracing::info;
use winit::dpi::PhysicalPosition;

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn begin_brush(&mut self) {
        if !self.demo_mode.is_scatter() {
            return;
        }
        let Some(brush_start) = self.plot_local_cursor_point() else {
            return;
        };

        self.last_drag_position = None;
        self.scatter.brush_drag_start = Some(brush_start);
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
        self.publish_scatter_active_selection();
        self.scatter.selection_summary = self
            .scatter
            .active_brush_selection
            .and_then(|selection| self.scatter_selection_summary_snapshot(selection));
        self.build_selection_evidence();
        self.build_selection_drilldown();
        self.rebuild_active_comparison();
        self.log_selection_summary("finalized");
        self.log_selection_evidence();
        self.log_selection_drilldown();
        self.scatter.brush_drag_start = None;
        self.scatter.active_brush_drag = None;
        self.update_window_title();
        self.request_redraw();
    }

    pub(crate) fn clear_brush(&mut self) {
        let had_selection = self.brush_is_active()
            || self.scatter.active_brush_selection.is_some()
            || self.scatter.active_brush_drag.is_some()
            || self.scatter.selection_summary.is_some()
            || self.scatter.selection_evidence.is_some()
            || self.scatter.selection_drilldown.is_some();
        self.scatter.active_brush_drag = None;
        self.scatter.active_brush_selection = None;
        self.scatter.selection_summary = None;
        self.scatter.selection_evidence = None;
        self.scatter.selection_drilldown = None;
        self.scatter.brush_drag_start = None;
        self.clear_active_selection();
        self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
        self.update_window_title();
        self.request_redraw();

        if had_selection {
            info!("RawScope scatter brush cleared");
        }
    }

    pub(crate) fn cancel_brush_gesture(&mut self) {
        self.scatter.brush_drag_start = None;
        self.scatter.active_brush_drag = None;
        self.request_redraw();
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

        let Some(screen_size) = self.plot_screen_size() else {
            return;
        };
        let Some(brush_end) = self.clamped_plot_local_point(cursor_position) else {
            return;
        };
        let brush_start = brush_drag_start;
        let next_drag = ScatterBrushDrag::from_screen_points(brush_start, brush_end, screen_size);

        self.scatter.active_brush_drag = next_drag;
        self.scatter.active_brush_selection =
            next_drag.and_then(|drag| drag.finalize(screen_size, viewport));
        self.scatter.selection_summary = self
            .scatter
            .active_brush_selection
            .and_then(|selection| self.scatter_selection_summary(selection));
        self.scatter.selection_evidence = None;
        self.scatter.selection_drilldown = None;
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

        let Some(screen_size) = self.plot_screen_size() else {
            return;
        };
        self.scatter.active_brush_selection = active_drag.finalize(screen_size, viewport);
        self.scatter.selection_summary = self
            .scatter
            .active_brush_selection
            .and_then(|selection| self.scatter_selection_summary(selection));
    }

    fn build_selection_evidence(&mut self) {
        if self.scatter_filters.is_active() {
            self.scatter.selection_evidence = None;
            return;
        }
        let Some(selection) = self.scatter.active_brush_selection else {
            self.scatter.selection_evidence = None;
            return;
        };
        let Some(dataset_metadata) = self.workbench_state.dataset_metadata.clone() else {
            self.scatter.selection_evidence = None;
            return;
        };

        self.scatter.selection_evidence = Some(ScatterSelectionEvidence::from_points(
            &self.scatter.points,
            ScatterSelectionGeometry {
                x_range: selection.x_range,
                y_range: selection.y_range,
            },
            dataset_metadata,
            self.scatter_evidence_row_count(),
            SelectionEvidenceConfig::default(),
        ));
    }

    fn build_selection_drilldown(&mut self) {
        let Some(selection) = self.scatter.active_brush_selection else {
            self.scatter.selection_drilldown = None;
            return;
        };

        let snapshot = self
            .workbench_state
            .active_selection
            .as_ref()
            .and_then(|selection| selection.snapshot.as_ref())
            .cloned();
        self.scatter.selection_drilldown = snapshot
            .as_ref()
            .map(|snapshot| {
                scatter_selection_drilldown_snapshot(
                    &self.scatter.points,
                    snapshot,
                    self.scatter.source_rows.as_ref(),
                    rawscope_render::DrilldownConfig::default(),
                )
            })
            .or_else(|| {
                self.scatter_filters.cohort_snapshot.as_ref().map(|cohort| {
                    let mask = cohort.filter_mask();
                    scatter_selection_drilldown_masked(
                        &self.scatter.points,
                        &mask,
                        selection,
                        self.scatter.source_rows.as_ref(),
                        rawscope_render::DrilldownConfig::default(),
                    )
                    .expect("cohort snapshot remains aligned with scatter points")
                })
            })
            .or_else(|| {
                self.scatter_filters
                    .evaluation
                    .as_ref()
                    .map(|evaluation| {
                        scatter_selection_drilldown_masked(
                            &self.scatter.points,
                            &evaluation.mask,
                            selection,
                            self.scatter.source_rows.as_ref(),
                            rawscope_render::DrilldownConfig::default(),
                        )
                        .expect("filter evaluation remains aligned with scatter points")
                    })
                    .or_else(|| {
                        Some(scatter_selection_drilldown(
                            &self.scatter.points,
                            selection,
                            self.scatter.source_rows.as_ref(),
                            rawscope_render::DrilldownConfig::default(),
                        ))
                    })
            });
    }

    fn scatter_selection_summary(
        &self,
        selection: ScatterBrushSelection,
    ) -> Option<SelectedRegionSummary> {
        self.scatter_filters
            .cohort_snapshot
            .as_ref()
            .map(|cohort| {
                let mask = cohort.filter_mask();
                selected_region_summary_masked(&self.scatter.points, &mask, selection)
                    .expect("cohort snapshot remains aligned with scatter points")
            })
            .or_else(|| {
                self.scatter_filters.evaluation.as_ref().map(|evaluation| {
                    selected_region_summary_masked(
                        &self.scatter.points,
                        &evaluation.mask,
                        selection,
                    )
                    .expect("filter evaluation remains aligned with scatter points")
                })
            })
            .or_else(|| {
                Some(SelectedRegionSummary::from_points(
                    &self.scatter.points,
                    selection,
                ))
            })
    }

    fn scatter_selection_summary_snapshot(
        &self,
        selection: ScatterBrushSelection,
    ) -> Option<SelectedRegionSummary> {
        let snapshot = self
            .workbench_state
            .active_selection
            .as_ref()
            .and_then(|active| active.snapshot.as_ref())?;
        let total_row_count = self.scatter_filters.cohort_snapshot.as_ref().map_or_else(
            || {
                self.scatter_filters
                    .evaluation
                    .as_ref()
                    .map_or(self.scatter.points.len(), |evaluation| {
                        evaluation.included_count
                    })
            },
            |cohort| cohort.included_row_count() as usize,
        );
        Some(selected_region_summary_snapshot(
            &self.scatter.points,
            snapshot,
            selection,
            total_row_count,
        ))
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

    fn log_selection_drilldown(&self) {
        let Some(drilldown) = &self.scatter.selection_drilldown else {
            return;
        };

        info!(
            selected_row_count = drilldown.selected_row_count,
            displayed_row_count = drilldown.displayed_row_count,
            sampled = drilldown.rows_are_sampled,
            column_count = drilldown.columns.len(),
            "RawScope selection drilldown prepared"
        );
    }
}

#[expect(
    clippy::field_reassign_with_default,
    reason = "fixtures intentionally mutate scenario-specific fields"
)]
#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};
    use rawscope_core::{F32Range, RowId};
    use rawscope_data::{
        generate_synthetic_points, ScatterPointKind, ScatterPointRecord, SyntheticPointConfig,
    };
    use rawscope_render::{PlotRectPx, ScatterBrushSelection, SelectionDrilldown};
    use winit::dpi::PhysicalPosition;

    use super::*;
    use crate::{
        demo::DemoMode,
        ui_plot_surface::{PlotAxisLayout, PlotSurfaceLayout},
    };

    fn app_with_plot() -> WorkbenchApp {
        WorkbenchApp {
            demo_mode: DemoMode::Scatter,
            plot_surface: Some(PlotSurfaceLayout {
                logical_rect: Rect::from_min_max(pos2(50.0, 25.0), pos2(250.0, 125.0)),
                physical_rect: PlotRectPx::try_new(100, 50, 400, 200, 800, 600).unwrap(),
                axis_layout: PlotAxisLayout {
                    outer_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(262.0, 163.0)),
                    plot_rect: Rect::from_min_max(pos2(50.0, 25.0), pos2(250.0, 125.0)),
                },
            }),
            scatter: crate::app::ScatterWorkbenchState {
                viewport: Some(rawscope_render::ScatterViewport::new(
                    F32Range::new(0.0, 100.0),
                    F32Range::new(0.0, 100.0),
                )),
                ..crate::app::ScatterWorkbenchState::default()
            },
            ..WorkbenchApp::default()
        }
    }

    #[test]
    fn brush_starts_require_plot_but_active_release_finalizes_outside() {
        let mut app = app_with_plot();
        app.cursor_position = Some(PhysicalPosition::new(99.0, 150.0));

        app.begin_brush();

        assert!(!app.brush_is_active());

        app.cursor_position = Some(PhysicalPosition::new(200.0, 100.0));
        app.begin_brush();
        app.update_brush_to_cursor(PhysicalPosition::new(700.0, 500.0));
        assert!(app.scatter.active_brush_selection.is_some());

        app.end_brush();

        assert!(!app.brush_is_active());
        assert!(app.scatter.active_brush_drag.is_none());
        assert!(app.scatter.active_brush_selection.is_some());
    }

    #[test]
    fn build_selection_drilldown_populates_scatter_fallback_rows() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.points = vec![
            ScatterPointRecord {
                row_id: RowId(3),
                x: 30.0,
                y: 40.0,
                kind: ScatterPointKind::Unclassified,
            },
            ScatterPointRecord {
                row_id: RowId(1),
                x: 10.0,
                y: 20.0,
                kind: ScatterPointKind::Unclassified,
            },
        ];
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(0.0, 50.0),
            y_range: F32Range::new(0.0, 50.0),
        });

        app.build_selection_drilldown();

        let drilldown = app
            .scatter
            .selection_drilldown
            .expect("drilldown should exist");
        assert_eq!(drilldown.selected_row_count, 2);
        assert_eq!(drilldown.displayed_row_count, 2);
        assert!(!drilldown.rows_are_sampled);
        assert_eq!(drilldown.columns[0].name, "row_id");
        assert_eq!(drilldown.rows[0].row_id, RowId(1));
        assert_eq!(
            drilldown.rows[0].values,
            vec![
                "1".to_string(),
                "10.000000".to_string(),
                "20.000000".to_string(),
                "unclassified".to_string(),
            ]
        );
    }

    #[test]
    fn clear_brush_clears_scatter_drilldown() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.workbench_state.dataset_identity =
            Some(generate_synthetic_points(SyntheticPointConfig::new(42, 1)).identity);
        app.scatter.points = vec![ScatterPointRecord {
            row_id: RowId(1),
            x: 10.0,
            y: 20.0,
            kind: ScatterPointKind::Unclassified,
        }];
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(0.0, 50.0),
            y_range: F32Range::new(0.0, 50.0),
        });
        app.publish_scatter_active_selection();
        app.scatter.selection_drilldown = Some(SelectionDrilldown {
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

        app.clear_brush();

        assert!(app.workbench_state.active_selection.is_none());
        assert!(app.scatter.selection_drilldown.is_none());
    }
}
