//! Immutable filtered selection membership for current render consumers.

use std::sync::Arc;

use rawscope_core::{RowId, SelectionId, ViewId};
use rawscope_data::{FilterMask, FilterRevision, ScatterPointRecord, TimelineEventRecord};

use crate::{ScatterBrushSelection, TimelineBrushSelection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSnapshot {
    selection_id: SelectionId,
    filter_revision: FilterRevision,
    view_id: ViewId,
    row_ids: Arc<[RowId]>,
    selected_bins: Arc<[u32]>,
    samples: Arc<[RowId]>,
}

impl SelectionSnapshot {
    pub fn from_filtered_points(
        selection_id: SelectionId,
        points: &[ScatterPointRecord],
        mask: &FilterMask,
        brush: ScatterBrushSelection,
        grid_width: u32,
        grid_height: u32,
    ) -> Result<Self, &'static str> {
        if points.len() != mask.len() {
            return Err("selection points and filter mask are misaligned");
        }
        if grid_width == 0 || grid_height == 0 {
            return Err("selection grid dimensions must be positive");
        }
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
        row_ids.sort_unstable();
        row_ids.dedup();
        bins.sort_unstable();
        bins.dedup();
        Ok(Self {
            selection_id,
            filter_revision: FilterRevision::default(),
            view_id: ViewId(0),
            row_ids: row_ids.into(),
            selected_bins: bins.into(),
            samples: Arc::from([]),
        })
    }

    pub fn from_filtered_events(
        selection_id: SelectionId,
        events: &[TimelineEventRecord],
        mask: &FilterMask,
        selection: TimelineBrushSelection,
        lane_count: u32,
        grid_width: u32,
        grid_height: u32,
    ) -> Result<Self, &'static str> {
        if events.len() != mask.len() {
            return Err("selection events and filter mask are misaligned");
        }
        if lane_count == 0 || grid_width == 0 || grid_height == 0 {
            return Err("selection timeline dimensions must be positive");
        }
        let span = u128::from(selection.time_range.span());
        if span == 0 {
            return Err("selection timeline range must have positive span");
        }
        let mut row_ids = Vec::new();
        let mut bins = Vec::new();
        for (event, included) in events.iter().zip(mask.as_gpu_u32_slice()) {
            if *included != 1 || !selection.contains_event(event) {
                continue;
            }
            row_ids.push(event.row_id);
            let x = if event.timestamp == selection.time_range.max {
                grid_width - 1
            } else {
                let offset = u128::from(event.timestamp - selection.time_range.min);
                u32::try_from(offset * u128::from(grid_width) / span)
                    .unwrap_or(grid_width - 1)
                    .min(grid_width - 1)
            };
            let y = (u64::from(event.lane) * u64::from(grid_height) / u64::from(lane_count))
                .try_into()
                .unwrap_or(grid_height - 1)
                .min(grid_height - 1);
            bins.push(y * grid_width + x);
        }
        row_ids.sort_unstable();
        row_ids.dedup();
        bins.sort_unstable();
        bins.dedup();
        Ok(Self {
            selection_id,
            filter_revision: FilterRevision::default(),
            view_id: ViewId(0),
            row_ids: row_ids.into(),
            selected_bins: bins.into(),
            samples: Arc::from([]),
        })
    }

    pub fn with_context(
        mut self,
        filter_revision: FilterRevision,
        view_id: ViewId,
        sample_limit: usize,
    ) -> Result<Self, &'static str> {
        if sample_limit == 0 {
            return Err("selection sample limit must be positive");
        }
        self.filter_revision = filter_revision;
        self.view_id = view_id;
        self.samples = self.row_ids.iter().copied().take(sample_limit).collect();
        Ok(self)
    }

    pub fn selection_id(&self) -> SelectionId {
        self.selection_id
    }
    pub fn filter_revision(&self) -> FilterRevision {
        self.filter_revision
    }
    pub fn view_id(&self) -> ViewId {
        self.view_id
    }
    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }
    pub fn selected_bins(&self) -> &[u32] {
        &self.selected_bins
    }
    pub fn selected_count(&self) -> usize {
        self.row_ids.len()
    }
    pub fn selected_count_u64(&self) -> u64 {
        self.row_ids.len() as u64
    }
    pub fn samples(&self) -> &[RowId] {
        &self.samples
    }
}

#[cfg(test)]
mod tests {
    use crate::TimelineLaneRange;

    use super::*;
    use rawscope_core::{F32Range, RowId, SelectionId, ViewId};
    use rawscope_data::{FilterMask, ScatterPointKind, TimelineEventKind, TimelineEventRecord};

    #[test]
    fn filtered_snapshot_keeps_unsampled_bins_from_complete_membership() {
        let points = vec![
            ScatterPointRecord {
                row_id: RowId(0),
                x: 0.1,
                y: 0.1,
                kind: ScatterPointKind::Unclassified,
            },
            ScatterPointRecord {
                row_id: RowId(1),
                x: 0.9,
                y: 0.9,
                kind: ScatterPointKind::Unclassified,
            },
        ];
        let brush = ScatterBrushSelection {
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
        };
        let snapshot = SelectionSnapshot::from_filtered_points(
            SelectionId(1),
            &points,
            &FilterMask::all_included(2),
            brush,
            2,
            2,
        )
        .unwrap();
        assert_eq!(snapshot.row_ids(), &[RowId(0), RowId(1)]);
        assert_eq!(snapshot.selected_bins(), &[1, 2]);
    }

    #[test]
    fn timeline_snapshot_applies_filter_before_ids_and_bins() {
        let events = vec![
            TimelineEventRecord {
                row_id: RowId(0),
                timestamp: 10,
                lane: 0,
                value: 1.0,
                kind: TimelineEventKind::Unclassified,
            },
            TimelineEventRecord {
                row_id: RowId(1),
                timestamp: 90,
                lane: 1,
                value: 1.0,
                kind: TimelineEventKind::Unclassified,
            },
        ];
        let selection = TimelineBrushSelection {
            time_range: rawscope_core::U64Range::new(0, 100),
            lane_range: TimelineLaneRange::new(0, 2),
        };
        let snapshot = SelectionSnapshot::from_filtered_events(
            SelectionId(2),
            &events,
            &FilterMask::from_u32(vec![1, 0]),
            selection,
            2,
            2,
            2,
        )
        .unwrap();
        assert_eq!(snapshot.row_ids(), &[RowId(0)]);
        assert_eq!(snapshot.selected_bins(), &[0]);
    }

    #[test]
    fn snapshot_context_bounds_samples_without_changing_complete_bins() {
        let points = vec![
            ScatterPointRecord {
                row_id: RowId(0),
                x: 0.1,
                y: 0.1,
                kind: ScatterPointKind::Unclassified,
            },
            ScatterPointRecord {
                row_id: RowId(1),
                x: 0.9,
                y: 0.9,
                kind: ScatterPointKind::Unclassified,
            },
        ];
        let snapshot = SelectionSnapshot::from_filtered_points(
            SelectionId(3),
            &points,
            &FilterMask::all_included(2),
            ScatterBrushSelection {
                x_range: F32Range::new(0.0, 1.0),
                y_range: F32Range::new(0.0, 1.0),
            },
            2,
            2,
        )
        .unwrap()
        .with_context(FilterRevision(7), ViewId(9), 1)
        .unwrap();
        assert_eq!(snapshot.filter_revision(), FilterRevision(7));
        assert_eq!(snapshot.view_id(), ViewId(9));
        assert_eq!(snapshot.samples(), &[RowId(0)]);
        assert_eq!(snapshot.selected_bins(), &[1, 2]);
    }
}
