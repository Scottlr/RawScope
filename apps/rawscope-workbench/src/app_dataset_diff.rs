//! Workbench-owned dataset diff loading and state helpers.

use std::{error::Error, io, path::Path};

use rawscope_data::{load_scatter_dataset, load_timeline_dataset, LoadedSourceTable};
use rawscope_render::dataset_diff_summary;

use crate::{app::WorkbenchApp, ui::WorkbenchSurface};

impl WorkbenchApp {
    pub(crate) fn dataset_diff_is_available(&self) -> bool {
        self.workbench_state.dataset_diff_summary.is_some()
    }

    pub(crate) fn show_dataset_diff_surface(&mut self) {
        if self.workbench_state.dataset_diff_summary.is_some() {
            self.workbench_state.visible_surface = WorkbenchSurface::DatasetDiff;
            self.clear_active_comparison();
            self.request_redraw();
            self.update_window_title();
        }
    }

    pub(crate) fn clear_dataset_diff_state(&mut self) {
        self.workbench_state.comparison_source_rows = None;
        self.workbench_state.dataset_diff_summary = None;
        if self.workbench_state.visible_surface == WorkbenchSurface::DatasetDiff {
            self.workbench_state.visible_surface = WorkbenchSurface::Primary;
        }
    }

    pub(crate) fn set_comparison_source_rows(
        &mut self,
        comparison_source_rows: Option<LoadedSourceTable>,
    ) {
        self.workbench_state.comparison_source_rows = comparison_source_rows;
        self.workbench_state.dataset_diff_summary = self
            .current_source_rows()
            .zip(self.workbench_state.comparison_source_rows.as_ref())
            .map(|(before, after)| dataset_diff_summary(before, after));

        if self.workbench_state.dataset_diff_summary.is_none()
            && self.workbench_state.visible_surface == WorkbenchSurface::DatasetDiff
        {
            self.workbench_state.visible_surface = WorkbenchSurface::Primary;
        }
    }

    pub(crate) fn inspect_dataset_diff_missingness_column(&mut self, column_name: &str) {
        let Some(column_index) = self.current_source_column_index(column_name) else {
            return;
        };

        self.select_missingness_column(column_index);
    }

    pub(crate) fn load_scatter_comparison_source_rows(
        &self,
        x_column: &str,
        y_column: &str,
        limit: Option<usize>,
    ) -> Result<Option<LoadedSourceTable>, Box<dyn Error>> {
        let Some(path) = self.compare_input.as_ref() else {
            return Ok(None);
        };

        let dataset = load_scatter_dataset(path, x_column, y_column, limit).map_err(|source| {
            Box::new(comparison_load_error(path, source.to_string())) as Box<dyn Error>
        })?;
        Ok(Some(dataset.source_rows))
    }

    pub(crate) fn load_timeline_comparison_source_rows(
        &self,
        time_column: &str,
        lane_column: &str,
        limit: Option<usize>,
    ) -> Result<Option<LoadedSourceTable>, Box<dyn Error>> {
        let Some(path) = self.compare_input.as_ref() else {
            return Ok(None);
        };

        let dataset =
            load_timeline_dataset(path, time_column, lane_column, limit).map_err(|source| {
                Box::new(comparison_load_error(path, source.to_string())) as Box<dyn Error>
            })?;
        Ok(Some(dataset.source_rows))
    }
}

fn comparison_load_error(path: &Path, message: String) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "failed to load comparison input '{}': {message}",
            path.display()
        ),
    )
}

#[expect(
    clippy::field_reassign_with_default,
    reason = "fixtures intentionally mutate scenario-specific fields"
)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::{DemoMode, PointCountPreset};
    use rawscope_core::RowId;
    use rawscope_data::{LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow};

    fn source_table(columns: &[(&str, LoadedColumnKind)], rows: &[&[&str]]) -> LoadedSourceTable {
        LoadedSourceTable {
            columns: columns
                .iter()
                .map(|(name, kind)| LoadedColumnSchema {
                    name: (*name).to_string(),
                    kind: *kind,
                })
                .collect(),
            rows: rows
                .iter()
                .enumerate()
                .map(|(row_index, values)| LoadedSourceRow {
                    row_id: RowId(row_index as u64),
                    values: values.iter().map(|value| (*value).to_string()).collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn set_comparison_source_rows_builds_summary_from_current_dataset() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.source_rows = Some(source_table(
            &[("alpha", LoadedColumnKind::String)],
            &[&[""], &["value"]],
        ));

        app.set_comparison_source_rows(Some(source_table(
            &[
                ("alpha", LoadedColumnKind::String),
                ("beta", LoadedColumnKind::Integer),
            ],
            &[&["", "1"], &["", "2"], &["value", "3"]],
        )));

        let summary = app
            .workbench_state
            .dataset_diff_summary
            .as_ref()
            .expect("dataset diff summary should exist");
        assert_eq!(summary.before_row_count, 2);
        assert_eq!(summary.after_row_count, 3);
        assert_eq!(summary.row_count_delta, 1);
        assert!(summary.columns.iter().any(|column| column.name == "beta"));
    }

    #[test]
    fn inspect_dataset_diff_missingness_column_reuses_missingness_selection() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.source_rows = Some(source_table(
            &[
                ("alpha", LoadedColumnKind::String),
                ("beta", LoadedColumnKind::String),
            ],
            &[&["", "ok"], &["value", ""], &[" ", "ok"]],
        ));
        app.rebuild_missingness_state();
        app.set_comparison_source_rows(Some(source_table(
            &[
                ("alpha", LoadedColumnKind::String),
                ("beta", LoadedColumnKind::String),
            ],
            &[&["value", "ok"], &["", ""], &["", "ok"]],
        )));

        app.inspect_dataset_diff_missingness_column("alpha");

        assert_eq!(
            app.workbench_state.visible_surface,
            WorkbenchSurface::Missingness
        );
        let summary = app
            .missingness
            .selection_summary
            .as_ref()
            .expect("missingness selection summary should exist");
        assert_eq!(summary.column_names, vec!["alpha".to_string()]);
        assert_eq!(summary.selected_missing_count, 2);
        assert_eq!(summary.selected_row_ids, vec![RowId(0), RowId(2)]);
    }

    #[test]
    fn preset_switch_clears_loaded_dataset_diff_state() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.source_rows = Some(source_table(
            &[("alpha", LoadedColumnKind::String)],
            &[&["value"]],
        ));
        app.set_comparison_source_rows(Some(source_table(
            &[("alpha", LoadedColumnKind::String)],
            &[&[""]],
        )));
        app.workbench_state.visible_surface = WorkbenchSurface::DatasetDiff;

        app.switch_point_preset(PointCountPreset {
            key_label: "2",
            row_count: 200_000,
        });

        assert!(app.workbench_state.comparison_source_rows.is_none());
        assert!(app.workbench_state.dataset_diff_summary.is_none());
        assert_eq!(
            app.workbench_state.visible_surface,
            WorkbenchSurface::Primary
        );
    }
}
