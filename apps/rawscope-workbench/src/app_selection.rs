//! Shared linked-selection publishing for the workbench.

use rawscope_core::{CoreLaneRange, SelectionId, ViewId, VisualSelection, VisualSelectionGeometry};
use rawscope_data::{DatasetIdentity, FilterMask};
use rawscope_render::SelectionSnapshot;

use crate::app::WorkbenchApp;

const SCATTER_VIEW_ID: ViewId = ViewId(1);
const TIMELINE_VIEW_ID: ViewId = ViewId(2);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActiveLinkedSelection {
    pub(crate) visual_selection: VisualSelection,
    pub(crate) dataset_identity: DatasetIdentity,
    pub(crate) snapshot: Option<SelectionSnapshot>,
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
            .evaluation
            .as_ref()
            .map(|evaluation| evaluation.mask.clone())
            .unwrap_or_else(|| FilterMask::all_included(self.scatter.points.len()));
        let selection_id = self.next_selection_id();
        let snapshot = SelectionSnapshot::from_filtered_points(
            selection_id,
            &self.scatter.points,
            &filter_mask,
            selection,
            256,
            256,
        )
        .ok();
        let selected_row_ids = snapshot
            .as_ref()
            .map(|snapshot| snapshot.row_ids().to_vec())
            .unwrap_or_default();

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
            snapshot,
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
        let snapshot = SelectionSnapshot::from_filtered_events(
            selection_id,
            &self.timeline.events,
            &filter_mask,
            selection,
            lane_count,
            256,
            256,
        )
        .ok();
        let selected_row_ids = snapshot
            .as_ref()
            .map(|snapshot| snapshot.row_ids().to_vec())
            .unwrap_or_default();

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
            snapshot,
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
        ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
    };
    use rawscope_render::{ScatterBrushSelection, TimelineBrushSelection, TimelineLaneRange};

    use super::*;
    use crate::demo::DemoMode;

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
}
