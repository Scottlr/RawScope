//! Workbench-owned comparison state for finalized visual selections.

use rawscope_data::DatasetIdentity;
use rawscope_render::MissingnessGrid;
use rawscope_render::{
    missingness_selection_comparison, scatter_selection_comparison, timeline_selection_comparison,
    MissingnessSelectionComparison, ScatterSelectionComparison, TimelineSelectionComparison,
};

use crate::{app::WorkbenchApp, demo::DemoMode, ui::WorkbenchSurface};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WorkbenchComparison {
    Scatter(ScatterSelectionComparison),
    Timeline(TimelineSelectionComparison),
    Missingness(MissingnessSelectionComparison),
}

impl WorkbenchApp {
    pub(crate) fn rebuild_active_comparison(&mut self) {
        self.active_comparison = match self.visible_surface {
            WorkbenchSurface::Primary => match self.demo_mode {
                DemoMode::Scatter => self.scatter_comparison(),
                DemoMode::Timeline => self.timeline_comparison(),
            },
            WorkbenchSurface::Missingness => self.missingness_comparison(),
            WorkbenchSurface::DatasetDiff => None,
        };
    }

    pub(crate) fn clear_active_comparison(&mut self) {
        self.active_comparison = None;
    }

    fn scatter_comparison(&self) -> Option<WorkbenchComparison> {
        let active_selection = self.active_selection.as_ref()?;
        if !self.datasets_match(&active_selection.dataset_identity) {
            return None;
        }
        let summary = self.scatter.selection_summary?;
        Some(WorkbenchComparison::Scatter(scatter_selection_comparison(
            &self.scatter.points,
            summary,
        )))
    }

    fn timeline_comparison(&self) -> Option<WorkbenchComparison> {
        let active_selection = self.active_selection.as_ref()?;
        if !self.datasets_match(&active_selection.dataset_identity) {
            return None;
        }
        let summary = self.timeline.selection_summary.clone()?;
        let viewport = self.timeline.viewport?;
        Some(WorkbenchComparison::Timeline(
            timeline_selection_comparison(&self.timeline.events, viewport.lane_count(), &summary),
        ))
    }

    fn missingness_comparison(&self) -> Option<WorkbenchComparison> {
        let grid = self.missingness.grid.as_ref()?;
        let summary = self.missingness.selection_summary.as_ref()?;
        let baseline_missing_count = total_missing_count(grid);
        let baseline_total_count = total_cell_count(grid);
        Some(WorkbenchComparison::Missingness(
            missingness_selection_comparison(summary, baseline_missing_count, baseline_total_count),
        ))
    }

    fn datasets_match(&self, dataset_identity: &DatasetIdentity) -> bool {
        self.dataset_identity
            .as_ref()
            .is_some_and(|identity| identity == dataset_identity)
    }
}

fn total_missing_count(grid: &MissingnessGrid) -> u64 {
    grid.cells
        .iter()
        .map(|cell| cell.missing_count as u64)
        .sum()
}

fn total_cell_count(grid: &MissingnessGrid) -> u64 {
    grid.cells.iter().map(|cell| cell.total_count as u64).sum()
}

#[expect(
    clippy::field_reassign_with_default,
    reason = "fixtures intentionally mutate scenario-specific fields"
)]
#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, RowId, U64Range};
    use rawscope_data::{
        generate_synthetic_events, generate_synthetic_points, LoadedColumnKind, LoadedColumnSchema,
        LoadedSourceRow, LoadedSourceTable, ScatterPointKind, ScatterPointRecord,
        SyntheticEventConfig, SyntheticPointCategory, SyntheticPointConfig, TimelineEventKind,
        TimelineEventRecord,
    };
    use rawscope_render::{
        ScatterBrushSelection, SelectedRegionSummary, TimelineBrushSelection, TimelineLaneRange,
        TimelineSelectionSummary,
    };

    use super::*;
    use crate::{
        demo::{DemoMode, PointCountPreset},
        ui::WorkbenchSurface,
    };

    fn scatter_points() -> Vec<ScatterPointRecord> {
        vec![
            ScatterPointRecord {
                row_id: RowId(1),
                x: 1.0,
                y: 1.0,
                kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Cluster),
            },
            ScatterPointRecord {
                row_id: RowId(2),
                x: 2.0,
                y: 2.0,
                kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Background),
            },
            ScatterPointRecord {
                row_id: RowId(3),
                x: 3.0,
                y: 3.0,
                kind: ScatterPointKind::Synthetic(SyntheticPointCategory::Outlier),
            },
        ]
    }

    fn timeline_events() -> Vec<TimelineEventRecord> {
        vec![
            TimelineEventRecord {
                row_id: RowId(1),
                timestamp: 10,
                lane: 0,
                value: 1.0,
                kind: TimelineEventKind::Synthetic(rawscope_data::SyntheticEventType::Background),
            },
            TimelineEventRecord {
                row_id: RowId(2),
                timestamp: 20,
                lane: 1,
                value: 2.0,
                kind: TimelineEventKind::Synthetic(rawscope_data::SyntheticEventType::Spike),
            },
            TimelineEventRecord {
                row_id: RowId(3),
                timestamp: 30,
                lane: 2,
                value: 3.0,
                kind: TimelineEventKind::Unclassified,
            },
        ]
    }

    fn local_source_rows() -> LoadedSourceTable {
        LoadedSourceTable {
            columns: vec![
                LoadedColumnSchema {
                    name: "alpha".to_string(),
                    kind: LoadedColumnKind::String,
                },
                LoadedColumnSchema {
                    name: "beta".to_string(),
                    kind: LoadedColumnKind::String,
                },
            ],
            rows: vec![
                LoadedSourceRow {
                    row_id: RowId(0),
                    values: vec!["".to_string(), "1".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(1),
                    values: vec!["2".to_string(), " ".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(2),
                    values: vec!["3".to_string(), "4".to_string()],
                },
            ],
        }
    }

    #[test]
    fn scatter_brush_rebuilds_comparison() {
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(42, 3));
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.visible_surface = WorkbenchSurface::Primary;
        app.dataset_identity = Some(dataset.identity);
        app.scatter.points = scatter_points();
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(0.0, 2.5),
            y_range: F32Range::new(0.0, 2.5),
        });
        app.scatter.selection_summary = Some(SelectedRegionSummary::from_points(
            &app.scatter.points,
            app.scatter.active_brush_selection.unwrap(),
        ));
        app.publish_scatter_active_selection();

        app.rebuild_active_comparison();

        assert!(matches!(
            app.active_comparison,
            Some(WorkbenchComparison::Scatter(_))
        ));
    }

    #[test]
    fn timeline_brush_rebuilds_comparison() {
        let dataset = generate_synthetic_events(SyntheticEventConfig::new(42, 3));
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Timeline;
        app.visible_surface = WorkbenchSurface::Primary;
        app.dataset_identity = Some(dataset.identity);
        app.timeline.events = timeline_events();
        app.timeline.viewport = Some(rawscope_render::TimelineViewport::new(
            U64Range::new(0, 100),
            4,
        ));
        app.timeline.selection_summary = Some(TimelineSelectionSummary::from_events(
            &app.timeline.events,
            TimelineBrushSelection {
                time_range: U64Range::new(0, 25),
                lane_range: TimelineLaneRange::new(0, 3),
            },
            app.timeline
                .viewport
                .as_ref()
                .expect("viewport should exist")
                .lane_count(),
        ));
        app.timeline.active_brush_selection = Some(TimelineBrushSelection {
            time_range: U64Range::new(0, 25),
            lane_range: TimelineLaneRange::new(0, 3),
        });
        app.publish_timeline_active_selection();

        app.rebuild_active_comparison();

        assert!(matches!(
            app.active_comparison,
            Some(WorkbenchComparison::Timeline(_))
        ));
    }

    #[test]
    fn missingness_selection_rebuilds_comparison() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.visible_surface = WorkbenchSurface::Missingness;
        app.scatter.source_rows = Some(local_source_rows());
        app.rebuild_missingness_state();
        app.select_missingness_cell(0, 0);

        app.rebuild_active_comparison();

        assert!(matches!(
            app.active_comparison,
            Some(WorkbenchComparison::Missingness(_))
        ));
    }

    #[test]
    fn clear_brush_clears_comparison() {
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(42, 3));
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.visible_surface = WorkbenchSurface::Primary;
        app.dataset_identity = Some(dataset.identity);
        app.scatter.points = scatter_points();
        app.scatter.selection_summary = Some(SelectedRegionSummary::from_points(
            &app.scatter.points,
            ScatterBrushSelection {
                x_range: F32Range::new(0.0, 2.5),
                y_range: F32Range::new(0.0, 2.5),
            },
        ));
        app.publish_scatter_active_selection();
        app.rebuild_active_comparison();

        app.clear_brush();

        assert!(app.active_comparison.is_none());
    }

    #[test]
    fn preset_switch_clears_comparison() {
        let dataset = generate_synthetic_points(SyntheticPointConfig::new(42, 20_000));
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.active_preset = PointCountPreset::default();
        app.visible_surface = WorkbenchSurface::Primary;
        app.dataset_identity = Some(dataset.identity);
        app.scatter.points = scatter_points();
        app.scatter.selection_summary = Some(SelectedRegionSummary::from_points(
            &app.scatter.points,
            ScatterBrushSelection {
                x_range: F32Range::new(0.0, 2.5),
                y_range: F32Range::new(0.0, 2.5),
            },
        ));
        app.publish_scatter_active_selection();
        app.rebuild_active_comparison();

        app.switch_point_preset(PointCountPreset::from_digit_key('2').unwrap());

        assert!(app.active_comparison.is_none());
    }
}
