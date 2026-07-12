//! Immutable filtered selection membership for current render consumers.

use std::sync::Arc;

use rawscope_core::{RowId, SelectionId};
use rawscope_data::{FilterMask, ScatterPointRecord};

use crate::ScatterBrushSelection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSnapshot {
    selection_id: SelectionId,
    row_ids: Arc<[RowId]>,
    selected_bins: Arc<[u32]>,
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
            row_ids: row_ids.into(),
            selected_bins: bins.into(),
        })
    }

    pub fn selection_id(&self) -> SelectionId {
        self.selection_id
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_core::{F32Range, RowId, SelectionId};
    use rawscope_data::{FilterMask, ScatterPointKind};

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
}
