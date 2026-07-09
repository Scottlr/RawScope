//! Shared linked-selection primitives for future interaction flows.

use crate::{F32Range, RowId, U64Range};

/// Identifies a user-driven selection in the visual exploration pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SelectionId(pub u64);

/// Identifies the source view that produced a linked visual selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

/// Shared selection kind independent of render- or app-owned brush structs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualSelectionKind {
    ScatterRect,
    TimelineRect,
}

/// Dependency-free half-open lane range for timeline-linked selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreLaneRange {
    pub start: u32,
    pub end_exclusive: u32,
}

impl CoreLaneRange {
    /// Creates a non-empty half-open lane range.
    pub fn new(start: u32, end_exclusive: u32) -> Self {
        assert!(
            end_exclusive > start,
            "lane range end must be greater than start"
        );
        Self {
            start,
            end_exclusive,
        }
    }
}

/// Shared geometry for a finalized linked selection.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualSelectionGeometry {
    ScatterRect {
        x_range: F32Range,
        y_range: F32Range,
    },
    TimelineRect {
        time_range: U64Range,
        lane_range: CoreLaneRange,
    },
}

/// A shared finalized selection contract that future views can consume.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualSelection {
    pub selection_id: SelectionId,
    pub source_view_id: ViewId,
    pub kind: VisualSelectionKind,
    pub geometry: VisualSelectionGeometry,
    pub selected_row_ids: Vec<RowId>,
}

impl VisualSelection {
    /// Creates a deterministic linked selection by sorting and deduplicating row ids.
    pub fn new(
        selection_id: SelectionId,
        source_view_id: ViewId,
        kind: VisualSelectionKind,
        geometry: VisualSelectionGeometry,
        mut selected_row_ids: Vec<RowId>,
    ) -> Self {
        selected_row_ids.sort_unstable_by_key(|row_id| row_id.0);
        selected_row_ids.dedup();

        Self {
            selection_id,
            source_view_id,
            kind,
            geometry,
            selected_row_ids,
        }
    }

    /// Returns the deterministic linked row count.
    pub fn selected_row_count(&self) -> usize {
        self.selected_row_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CoreLaneRange, SelectionId, ViewId, VisualSelection, VisualSelectionGeometry,
        VisualSelectionKind,
    };
    use crate::{F32Range, RowId, U64Range};

    #[test]
    fn visual_selection_sorts_and_deduplicates_row_ids() {
        let selection = VisualSelection::new(
            SelectionId(7),
            ViewId(2),
            VisualSelectionKind::ScatterRect,
            VisualSelectionGeometry::ScatterRect {
                x_range: F32Range::new(0.0, 1.0),
                y_range: F32Range::new(2.0, 3.0),
            },
            vec![RowId(9), RowId(2), RowId(9), RowId(1)],
        );

        assert_eq!(
            selection.selected_row_ids,
            vec![RowId(1), RowId(2), RowId(9)]
        );
        assert_eq!(selection.selected_row_count(), 3);
    }

    #[test]
    fn visual_selection_keeps_timeline_geometry() {
        let selection = VisualSelection::new(
            SelectionId(3),
            ViewId(4),
            VisualSelectionKind::TimelineRect,
            VisualSelectionGeometry::TimelineRect {
                time_range: U64Range::new(10, 20),
                lane_range: CoreLaneRange::new(1, 3),
            },
            vec![RowId(5)],
        );

        assert_eq!(selection.selection_id, SelectionId(3));
        assert_eq!(selection.source_view_id, ViewId(4));
        assert_eq!(selection.kind, VisualSelectionKind::TimelineRect);
        assert_eq!(selection.selected_row_count(), 1);
    }
}
