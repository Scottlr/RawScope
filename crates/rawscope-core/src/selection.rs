//! Shared linked-selection primitives for interaction and evidence flows.

use std::{error::Error, fmt, ops::Deref};

use crate::{F32Range, RowId, U64Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SelectionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

/// Legacy display label derived from `VisualSelectionGeometry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualSelectionKind {
    ScatterRect,
    TimelineRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreLaneRangeFields {
    pub start: u32,
    pub end_exclusive: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreLaneRangeError {
    EmptyOrReversed { start: u32, end_exclusive: u32 },
}

impl fmt::Display for CoreLaneRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyOrReversed {
                start,
                end_exclusive,
            } => write!(
                formatter,
                "lane range {start}..{end_exclusive} is empty or reversed"
            ),
        }
    }
}

impl Error for CoreLaneRangeError {}

/// Dependency-free half-open lane range for timeline-linked selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreLaneRange {
    fields: CoreLaneRangeFields,
}

impl CoreLaneRange {
    pub fn try_new(start: u32, end_exclusive: u32) -> Result<Self, CoreLaneRangeError> {
        if end_exclusive <= start {
            return Err(CoreLaneRangeError::EmptyOrReversed {
                start,
                end_exclusive,
            });
        }
        Ok(Self {
            fields: CoreLaneRangeFields {
                start,
                end_exclusive,
            },
        })
    }

    pub fn new(start: u32, end_exclusive: u32) -> Self {
        Self::try_new(start, end_exclusive).expect("validated core lane range")
    }

    pub const fn start(self) -> u32 {
        self.fields.start
    }

    pub const fn end_exclusive(self) -> u32 {
        self.fields.end_exclusive
    }
}

impl Deref for CoreLaneRange {
    type Target = CoreLaneRangeFields;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualSelectionError {
    RowIdsNotStrictlySorted { position: usize },
}

impl fmt::Display for VisualSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowIdsNotStrictlySorted { position } => write!(
                formatter,
                "selection row ids must be strictly sorted and unique at position {position}"
            ),
        }
    }
}

impl Error for VisualSelectionError {}

/// A finalized linked selection with geometry as its sole kind discriminator.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualSelection {
    selection_id: SelectionId,
    source_view_id: ViewId,
    geometry: VisualSelectionGeometry,
    selected_row_ids: Vec<RowId>,
}

impl VisualSelection {
    pub fn try_new(
        selection_id: SelectionId,
        source_view_id: ViewId,
        geometry: VisualSelectionGeometry,
        selected_row_ids: Vec<RowId>,
    ) -> Result<Self, VisualSelectionError> {
        for (position, pair) in selected_row_ids.windows(2).enumerate() {
            if pair[0] >= pair[1] {
                return Err(VisualSelectionError::RowIdsNotStrictlySorted { position });
            }
        }
        Ok(Self {
            selection_id,
            source_view_id,
            geometry,
            selected_row_ids,
        })
    }

    /// Sorts and deduplicates current producer output before validation.
    pub fn from_unsorted(
        selection_id: SelectionId,
        source_view_id: ViewId,
        geometry: VisualSelectionGeometry,
        mut selected_row_ids: Vec<RowId>,
    ) -> Self {
        selected_row_ids.sort_unstable_by_key(|row_id| row_id.0);
        selected_row_ids.dedup();
        Self::try_new(selection_id, source_view_id, geometry, selected_row_ids)
            .expect("sorted selection row ids are valid")
    }

    pub const fn selection_id(&self) -> SelectionId {
        self.selection_id
    }

    pub const fn source_view_id(&self) -> ViewId {
        self.source_view_id
    }

    pub fn geometry(&self) -> &VisualSelectionGeometry {
        &self.geometry
    }

    pub fn selected_row_ids(&self) -> &[RowId] {
        &self.selected_row_ids
    }

    pub const fn kind(&self) -> VisualSelectionKind {
        match self.geometry {
            VisualSelectionGeometry::ScatterRect { .. } => VisualSelectionKind::ScatterRect,
            VisualSelectionGeometry::TimelineRect { .. } => VisualSelectionKind::TimelineRect,
        }
    }

    pub fn selected_row_count(&self) -> usize {
        self.selected_row_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CoreLaneRange, SelectionId, ViewId, VisualSelection, VisualSelectionError,
        VisualSelectionGeometry, VisualSelectionKind,
    };
    use crate::{F32Range, RowId, U64Range};

    #[test]
    fn visual_selection_from_unsorted_rows_is_canonical() {
        let selection = VisualSelection::from_unsorted(
            SelectionId(7),
            ViewId(2),
            VisualSelectionGeometry::ScatterRect {
                x_range: F32Range::new(0.0, 1.0),
                y_range: F32Range::new(2.0, 3.0),
            },
            vec![RowId(9), RowId(2), RowId(9), RowId(1)],
        );

        assert_eq!(
            selection.selected_row_ids(),
            &[RowId(1), RowId(2), RowId(9)]
        );
        assert_eq!(selection.kind(), VisualSelectionKind::ScatterRect);
    }

    #[test]
    fn visual_selection_rejects_duplicate_rows_from_checked_constructor() {
        let result = VisualSelection::try_new(
            SelectionId(1),
            ViewId(1),
            VisualSelectionGeometry::TimelineRect {
                time_range: U64Range::new(10, 20),
                lane_range: CoreLaneRange::new(1, 3),
            },
            vec![RowId(1), RowId(1)],
        );
        assert_eq!(
            result,
            Err(VisualSelectionError::RowIdsNotStrictlySorted { position: 0 })
        );
    }
}
