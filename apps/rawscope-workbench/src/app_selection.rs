//! Shared linked-selection publishing for the workbench.

use rawscope_analysis::selection::SelectionSnapshot;
use rawscope_core::{CoreLaneRange, SelectionId, ViewId, VisualSelection, VisualSelectionGeometry};
use rawscope_data::{DatasetIdentity, FilterMask, ScatterPointRecord, TimelineEventRecord};

use crate::app::WorkbenchApp;

const SCATTER_VIEW_ID: ViewId = ViewId(1);
const TIMELINE_VIEW_ID: ViewId = ViewId(2);
const SELECTION_SAMPLE_LIMIT: usize = 10;

fn scatter_membership(
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    brush: rawscope_render::ScatterBrushSelection,
    grid_width: u32,
    grid_height: u32,
) -> (Vec<rawscope_core::RowId>, Vec<u32>) {
    let mut row_ids = Vec::new();
    let mut bins = Vec::new();
    for (point, included) in points.iter().zip(mask.as_gpu_u32_slice()) {
        if *included != 1 || !brush.contains_point(point) {
            continue;
        }
        row_ids.push(point.row_id);
        let x = (((point.x - brush.x_range.min) / brush.x_range.span()) * grid_width as f32)
            .floor()
            .clamp(0.0, (grid_width - 1) as f32) as u32;
        let y = (((brush.y_range.max - point.y) / brush.y_range.span()) * grid_height as f32)
            .floor()
            .clamp(0.0, (grid_height - 1) as f32) as u32;
        bins.push(y * grid_width + x);
    }
    (row_ids, bins)
}

fn timeline_membership(
    events: &[TimelineEventRecord],
    mask: &FilterMask,
    selection: rawscope_render::TimelineBrushSelection,
    lane_count: u32,
    grid_width: u32,
    grid_height: u32,
) -> (Vec<rawscope_core::RowId>, Vec<u32>) {
    let mut row_ids = Vec::new();
    let mut bins = Vec::new();
    let span = u128::from(selection.time_range.span());
    for (event, included) in events.iter().zip(mask.as_gpu_u32_slice()) {
        if *included != 1 || !selection.contains_event(event) {
            continue;
        }
        row_ids.push(event.row_id);
        let x = if event.timestamp == selection.time_range.max {
            grid_width.saturating_sub(1)
        } else {
            let offset = u128::from(event.timestamp - selection.time_range.min);
            u32::try_from(offset * u128::from(grid_width) / span)
                .unwrap_or(grid_width.saturating_sub(1))
                .min(grid_width.saturating_sub(1))
        };
        let y = (u64::from(event.lane) * u64::from(grid_height) / u64::from(lane_count))
            .try_into()
            .unwrap_or(grid_height.saturating_sub(1))
            .min(grid_height.saturating_sub(1));
        bins.push(y * grid_width + x);
    }
    (row_ids, bins)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActiveLinkedSelection {
    pub(crate) visual_selection: VisualSelection,
    pub(crate) dataset_identity: DatasetIdentity,
    pub(crate) analysis_snapshot: Option<SelectionSnapshot>,
}

impl WorkbenchApp {
    pub(crate) fn publish_scatter_active_selection(&mut self) {
        let Some(selection) = self.scatter.active_brush_selection else {
            self.clear_active_selection();
            return;
        };
        let Some(dataset_identity) = self.workbench_state.dataset_identity.clone() else {
            self.clear_active_selection();
            return;
        };

        let filter_mask = self
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .map(|snapshot| snapshot.filter_mask())
            .or_else(|| {
                self.scatter_filters
                    .evaluation
                    .as_ref()
                    .map(|evaluation| evaluation.mask.clone())
            })
            .unwrap_or_else(|| FilterMask::all_included(self.scatter.points.len()));
        let selection_id = self.next_selection_id();
        let (selected_row_ids, selected_bins) =
            scatter_membership(&self.scatter.points, &filter_mask, selection, 256, 256);
        let analysis_snapshot = self
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .and_then(|cohort| {
                SelectionSnapshot::from_parts(
                    cohort.dataset_generation(),
                    cohort.cohort_generation(),
                    selection_id,
                    selected_row_ids.iter().copied(),
                    selected_bins.iter().copied(),
                    SELECTION_SAMPLE_LIMIT,
                )
                .ok()
            });

        self.workbench_state.active_selection = Some(ActiveLinkedSelection {
            visual_selection: VisualSelection::from_unsorted(
                selection_id,
                SCATTER_VIEW_ID,
                VisualSelectionGeometry::ScatterRect {
                    x_range: selection.x_range,
                    y_range: selection.y_range,
                },
                selected_row_ids,
            ),
            dataset_identity,
            analysis_snapshot,
        });
    }

    pub(crate) fn publish_timeline_active_selection(&mut self) {
        let Some(selection) = self.timeline.active_brush_selection else {
            self.clear_active_selection();
            return;
        };
        let Some(dataset_identity) = self.workbench_state.dataset_identity.clone() else {
            self.clear_active_selection();
            return;
        };

        let lane_count = self
            .timeline
            .viewport
            .as_ref()
            .map_or(selection.lane_range.end_exclusive, |viewport| {
                viewport.lane_count()
            });
        let filter_mask = FilterMask::all_included(self.timeline.events.len());
        let selection_id = self.next_selection_id();
        let (selected_row_ids, _selected_bins) = timeline_membership(
            &self.timeline.events,
            &filter_mask,
            selection,
            lane_count,
            256,
            256,
        );

        let lane_range = match CoreLaneRange::try_new(
            selection.lane_range.start,
            selection.lane_range.end_exclusive,
        ) {
            Ok(lane_range) => lane_range,
            Err(_) => {
                self.clear_active_selection();
                return;
            }
        };

        self.workbench_state.active_selection = Some(ActiveLinkedSelection {
            visual_selection: VisualSelection::from_unsorted(
                selection_id,
                TIMELINE_VIEW_ID,
                VisualSelectionGeometry::TimelineRect {
                    time_range: selection.time_range,
                    lane_range,
                },
                selected_row_ids,
            ),
            dataset_identity,
            analysis_snapshot: None,
        });
    }

    pub(crate) fn clear_active_selection(&mut self) {
        self.workbench_state.active_selection = None;
        self.clear_active_comparison();
    }

    pub(crate) fn active_linked_selection_count(&self) -> usize {
        self.workbench_state
            .active_selection
            .as_ref()
            .map(|selection| selection.visual_selection.selected_row_count())
            .unwrap_or(0)
    }

    fn next_selection_id(&mut self) -> SelectionId {
        self.workbench_state.next_selection_id.0 += 1;
        self.workbench_state.next_selection_id
    }
}

#[expect(
    clippy::field_reassign_with_default,
    reason = "fixtures intentionally mutate scenario-specific fields"
)]
#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, RowId, U64Range};
    use rawscope_data::{
        LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable, ScatterPointKind,
        ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
    };
    use rawscope_evidence::TimelineLaneRange;
    use rawscope_render::{ScatterBrushSelection, TimelineBrushSelection};

    use super::*;
    use crate::demo::DemoMode;
    use crate::ui_filters::FilterAction;

    #[test]
    fn publish_scatter_active_selection_sorts_row_ids() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.workbench_state.dataset_identity = Some(
            rawscope_data::generate_synthetic_points(rawscope_data::SyntheticPointConfig::new(
                42, 2,
            ))
            .identity,
        );
        app.scatter.points = vec![
            ScatterPointRecord {
                row_id: RowId(7),
                x: 4.0,
                y: 4.0,
                kind: ScatterPointKind::Unclassified,
            },
            ScatterPointRecord {
                row_id: RowId(2),
                x: 2.0,
                y: 2.0,
                kind: ScatterPointKind::Unclassified,
            },
        ];
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(0.0, 10.0),
            y_range: F32Range::new(0.0, 10.0),
        });

        app.publish_scatter_active_selection();

        let selection = app
            .workbench_state
            .active_selection
            .as_ref()
            .expect("active selection should exist");
        assert_eq!(
            selection.visual_selection.selected_row_ids(),
            vec![RowId(2), RowId(7)]
        );
    }

    #[test]
    fn publish_timeline_active_selection_uses_incrementing_ids() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Timeline;
        app.workbench_state.dataset_identity = Some(
            rawscope_data::generate_synthetic_events(rawscope_data::SyntheticEventConfig::new(
                42, 2,
            ))
            .identity,
        );
        app.timeline.events = vec![
            TimelineEventRecord {
                row_id: RowId(3),
                timestamp: 10,
                lane: 0,
                value: 1.0,
                kind: TimelineEventKind::Unclassified,
            },
            TimelineEventRecord {
                row_id: RowId(1),
                timestamp: 12,
                lane: 1,
                value: 2.0,
                kind: TimelineEventKind::Unclassified,
            },
        ];
        app.timeline.active_brush_selection = Some(TimelineBrushSelection {
            time_range: U64Range::new(0, 20),
            lane_range: TimelineLaneRange::new(0, 2),
        });

        app.publish_timeline_active_selection();
        let first_selection_id = app
            .workbench_state
            .active_selection
            .as_ref()
            .expect("active selection should exist")
            .visual_selection
            .selection_id();
        app.publish_timeline_active_selection();
        let second_selection_id = app
            .workbench_state
            .active_selection
            .as_ref()
            .expect("active selection should exist")
            .visual_selection
            .selection_id();

        assert_eq!(first_selection_id, SelectionId(1));
        assert_eq!(second_selection_id, SelectionId(2));
    }

    #[test]
    fn publish_scatter_selection_uses_cohort_membership_not_compatibility_samples() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.workbench_state.dataset_identity = Some(
            rawscope_data::generate_synthetic_points(rawscope_data::SyntheticPointConfig::new(
                42, 2,
            ))
            .identity,
        );
        app.scatter.source_rows = Some(LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "winner".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: vec![
                LoadedSourceRow {
                    row_id: RowId(0),
                    values: vec!["white".into()],
                },
                LoadedSourceRow {
                    row_id: RowId(1),
                    values: vec!["black".into()],
                },
            ],
        });
        app.scatter.points = vec![
            ScatterPointRecord {
                row_id: RowId(0),
                x: 1.0,
                y: 1.0,
                kind: ScatterPointKind::Unclassified,
            },
            ScatterPointRecord {
                row_id: RowId(1),
                x: 2.0,
                y: 2.0,
                kind: ScatterPointKind::Unclassified,
            },
        ];
        app.initialize_scatter_filters();
        app.apply_scatter_filter_action(FilterAction::SetCategories {
            column_name: "winner".into(),
            included_values: vec!["white".into()],
            include_missing: false,
        });
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(0.0, 3.0),
            y_range: F32Range::new(0.0, 3.0),
        });

        app.publish_scatter_active_selection();

        let selection = app
            .workbench_state
            .active_selection
            .as_ref()
            .expect("filtered selection should be published");
        assert_eq!(
            selection.visual_selection.selected_row_ids(),
            vec![RowId(0)]
        );
    }
}
