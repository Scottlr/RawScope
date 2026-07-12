//! CPU-backed missingness state and selection handling for the workbench.

use rawscope_data::LoadedSourceTable;
use rawscope_render::{
    missingness_grid, missingness_selection_summary, MissingnessGrid, MissingnessSelection,
    MissingnessSelectionSummary,
};

use crate::{app::WorkbenchApp, ui::WorkbenchSurface};

/// Default number of row buckets used for the first missingness slice.
pub(crate) const DEFAULT_MISSINGNESS_ROW_BUCKET_COUNT: u32 = 16;

/// Workbench-owned CPU missingness state derived from retained local source rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingnessWorkbenchState {
    pub(crate) row_bucket_count: u32,
    pub(crate) grid: Option<MissingnessGrid>,
    pub(crate) selection: Option<MissingnessSelection>,
    pub(crate) selection_summary: Option<MissingnessSelectionSummary>,
}

impl Default for MissingnessWorkbenchState {
    fn default() -> Self {
        Self {
            row_bucket_count: DEFAULT_MISSINGNESS_ROW_BUCKET_COUNT,
            grid: None,
            selection: None,
            selection_summary: None,
        }
    }
}

impl WorkbenchApp {
    pub(crate) fn rebuild_missingness_state(&mut self) {
        let Some(source_rows) = self.current_source_rows() else {
            self.missingness = MissingnessWorkbenchState::default();
            self.visible_surface = WorkbenchSurface::Primary;
            return;
        };

        let next_grid = missingness_grid(source_rows, self.missingness.row_bucket_count);
        self.missingness.grid = Some(next_grid);
        self.clear_missingness_selection();
    }

    pub(crate) fn show_missingness_surface(&mut self) {
        if self.missingness.grid.is_some() {
            self.visible_surface = WorkbenchSurface::Missingness;
            self.clear_active_comparison();
            self.request_redraw();
            self.update_window_title();
        }
    }

    pub(crate) fn show_primary_surface(&mut self) {
        if self.visible_surface != WorkbenchSurface::Primary {
            self.visible_surface = WorkbenchSurface::Primary;
            self.rebuild_active_comparison();
            self.request_redraw();
            self.update_window_title();
        }
    }

    pub(crate) fn select_missingness_cell(&mut self, row_bucket: u32, column_index: u32) {
        let selection = MissingnessSelection::new(
            row_bucket,
            row_bucket.saturating_add(1),
            column_index,
            column_index.saturating_add(1),
        );
        self.apply_missingness_selection(selection, true);
    }

    pub(crate) fn select_missingness_column(&mut self, column_index: u32) {
        let row_bucket_count = self
            .missingness
            .grid
            .as_ref()
            .map(|grid| grid.row_bucket_count)
            .unwrap_or(self.missingness.row_bucket_count);
        let selection = MissingnessSelection::new(
            0,
            row_bucket_count,
            column_index,
            column_index.saturating_add(1),
        );

        self.apply_missingness_selection(selection, false);
    }

    pub(crate) fn clear_missingness_selection(&mut self) {
        self.missingness.selection = None;
        self.missingness.selection_summary = None;
        self.clear_active_comparison();
    }

    pub(crate) fn missingness_is_available(&self) -> bool {
        self.missingness.grid.is_some()
    }

    pub(crate) fn current_source_rows(&self) -> Option<&LoadedSourceTable> {
        match self.demo_mode {
            crate::demo::DemoMode::Scatter => self.scatter.source_rows.as_ref(),
            crate::demo::DemoMode::Timeline => self.timeline.source_rows.as_ref(),
        }
    }

    pub(crate) fn current_source_column_index(&self, column_name: &str) -> Option<u32> {
        self.current_source_rows()?
            .columns
            .iter()
            .position(|column| column.name == column_name)
            .and_then(|column_index| u32::try_from(column_index).ok())
    }

    fn apply_missingness_selection(
        &mut self,
        selection: MissingnessSelection,
        toggle_if_same_selection: bool,
    ) {
        let selection_is_already_active = self.missingness.selection == Some(selection);
        if toggle_if_same_selection && selection_is_already_active {
            self.clear_missingness_selection();
            self.request_redraw();
            self.update_window_title();
            return;
        }

        let row_bucket_count = self
            .missingness
            .grid
            .as_ref()
            .map(|grid| grid.row_bucket_count)
            .unwrap_or(self.missingness.row_bucket_count);
        let Some(next_summary) = self.current_source_rows().map(|source_rows| {
            missingness_selection_summary(source_rows, row_bucket_count, selection)
        }) else {
            self.clear_missingness_selection();
            return;
        };

        self.visible_surface = WorkbenchSurface::Missingness;
        self.missingness.selection = Some(selection);
        self.missingness.selection_summary = Some(next_summary);
        self.rebuild_active_comparison();
        self.request_redraw();
        self.update_window_title();
    }
}

#[expect(
    clippy::field_reassign_with_default,
    reason = "fixtures intentionally mutate scenario-specific fields"
)]
#[cfg(test)]
mod tests {
    use rawscope_core::RowId;
    use rawscope_data::{LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable};

    use super::*;
    use crate::demo::DemoMode;

    fn source_table_fixture() -> LoadedSourceTable {
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
                    values: vec!["".to_string(), "ok".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(1),
                    values: vec!["1".to_string(), " ".to_string()],
                },
            ],
        }
    }

    #[test]
    fn rebuild_missingness_state_uses_current_local_source_rows() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.source_rows = Some(source_table_fixture());

        app.rebuild_missingness_state();

        let grid = app.missingness.grid.expect("missingness grid should exist");
        assert_eq!(grid.column_count, 2);
        assert_eq!(grid.row_bucket_count, 2);
    }

    #[test]
    fn select_missingness_cell_builds_summary_from_selected_region() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.source_rows = Some(source_table_fixture());
        app.rebuild_missingness_state();

        app.select_missingness_cell(0, 0);

        let summary = app
            .missingness
            .selection_summary
            .as_ref()
            .expect("missingness summary should exist");
        assert_eq!(summary.selected_missing_count, 1);
        assert_eq!(summary.column_names, vec!["alpha".to_string()]);
        assert_eq!(summary.selected_row_ids, vec![RowId(0)]);
    }
}
