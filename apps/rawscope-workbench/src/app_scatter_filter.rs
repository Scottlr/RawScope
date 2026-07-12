//! Scatter filter catalog, evaluation, and cohort revision coordination.

use rawscope_data::{
    available_profile_filter_hints, build_visual_field_catalog, dataset_profile, evaluate_filters,
    DatasetFilter, DatasetProfileId, FilterEvaluation, FilterRevision, FilterSet,
    LoadedSourceTable, VisualFieldCatalog, VisualFieldCatalogConfig,
};

use crate::{app::WorkbenchApp, ui::ExportStatus, ui_filters::FilterAction};

const DEFAULT_EXPANDED_FILTER_COUNT: usize = 4;

#[derive(Debug, Clone, Default)]
pub(crate) struct ScatterFilterState {
    pub(crate) catalog: Option<VisualFieldCatalog>,
    pub(crate) filters: FilterSet,
    pub(crate) evaluation: Option<FilterEvaluation>,
    pub(crate) last_uploaded_revision: FilterRevision,
    pub(crate) visible_columns: Vec<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedFilterState {
    pub(crate) filters: FilterSet,
    pub(crate) evaluation: FilterEvaluation,
}

impl ScatterFilterState {
    pub(crate) fn initialize(
        &mut self,
        source: &LoadedSourceTable,
        profile_id: Option<DatasetProfileId>,
    ) {
        let catalog = build_visual_field_catalog(source, VisualFieldCatalogConfig::default());
        let visible_columns = profile_id
            .map(dataset_profile)
            .map(|profile| available_profile_filter_hints(profile, &catalog))
            .unwrap_or_default()
            .into_iter()
            .take(DEFAULT_EXPANDED_FILTER_COUNT)
            .map(|hint| hint.column_name.to_string())
            .collect();
        let filters = FilterSet::default();
        let evaluation = evaluate_filters(source, &catalog, &filters).ok();
        *self = Self {
            catalog: Some(catalog),
            filters,
            evaluation,
            last_uploaded_revision: FilterRevision::default(),
            visible_columns,
            error: None,
        };
    }

    pub(crate) fn is_active(&self) -> bool {
        self.filters.is_active()
    }
}

impl WorkbenchApp {
    pub(crate) fn initialize_scatter_filters(&mut self) {
        let Some(source) = self.scatter.source_rows.as_ref() else {
            self.scatter_filters = ScatterFilterState::default();
            return;
        };
        self.scatter_filters
            .initialize(source, self.active_dataset_profile);
    }

    pub(crate) fn apply_scatter_filter_action(&mut self, action: FilterAction) {
        let Some(catalog) = self.scatter_filters.catalog.as_ref() else {
            return;
        };
        if let FilterAction::AddColumn { column_name } = action {
            if catalog
                .fields
                .iter()
                .any(|field| field.column_name == column_name)
                && !self.scatter_filters.visible_columns.contains(&column_name)
            {
                self.scatter_filters.visible_columns.push(column_name);
                self.request_redraw();
            }
            return;
        }

        let prepared = match self.prepare_filter_action(action) {
            Ok(Some(prepared)) => prepared,
            Ok(None) => return,
            Err(err) => {
                self.scatter_filters.error = Some(err);
                self.request_redraw();
                return;
            }
        };
        let next_filters = prepared.filters.clone();
        let evaluation = prepared.evaluation;
        let difference_remains_available =
            next_filters.is_active() && evaluation.included_count > 0;
        let evaluation_revision = evaluation.revision;

        let upload_result = self
            .gpu
            .as_ref()
            .zip(self.scatter.density_renderer.as_mut())
            .map(|(gpu, renderer)| {
                renderer.update_filter_mask(gpu.queue(), &evaluation.mask, evaluation_revision)
            });
        if let Err(err) = upload_result.transpose() {
            self.scatter_filters.error = Some(err.to_string());
            return;
        }
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.difference_renderer.as_mut())
        {
            if let Err(err) =
                renderer.update_filter_mask(gpu.queue(), &evaluation.mask, evaluation_revision)
            {
                self.scatter_filters.error = Some(err.to_string());
                return;
            }
        }

        self.scatter_filters.last_uploaded_revision = evaluation_revision;
        self.scatter_filters.filters = next_filters;
        self.scatter_filters.evaluation = Some(evaluation);
        self.scatter_filters.error = None;
        if !difference_remains_available {
            self.scatter.density_mode = rawscope_render::ScatterDensityMode::AbsoluteDensity;
        }
        self.invalidate_scatter_inspection();
        self.invalidate_scatter_point_reveal();
        self.clear_brush();
        self.export_status = ExportStatus::Idle;
        self.render_schedule.request_exact_refine();
        self.request_redraw();
    }

    fn prepare_filter_action(
        &self,
        action: FilterAction,
    ) -> Result<Option<PreparedFilterState>, String> {
        let Some(source) = self.scatter.source_rows.as_ref() else {
            return Ok(None);
        };
        let Some(catalog) = self.scatter_filters.catalog.as_ref() else {
            return Ok(None);
        };
        if matches!(action, FilterAction::AddColumn { .. }) {
            return Ok(None);
        }
        let mut next_filters = self.scatter_filters.filters.clone();
        match action {
            FilterAction::SetNumericRange {
                column_name,
                min_inclusive,
                max_inclusive,
                include_missing,
            } => next_filters.replace_for_column(DatasetFilter::NumericRange {
                column_name,
                min_inclusive,
                max_inclusive,
                include_missing,
            }),
            FilterAction::SetCategories {
                column_name,
                included_values,
                include_missing,
            } => next_filters.replace_for_column(DatasetFilter::Categories {
                column_name,
                included_values,
                include_missing,
            }),
            FilterAction::RemoveColumn { column_name } => {
                next_filters.remove_column(&column_name);
            }
            FilterAction::ClearAll => next_filters.clear(),
            FilterAction::AddColumn { .. } => unreachable!("handled above"),
        }
        if next_filters == self.scatter_filters.filters {
            return Ok(None);
        }
        let evaluation =
            evaluate_filters(source, catalog, &next_filters).map_err(|err| err.to_string())?;
        Ok(Some(PreparedFilterState {
            filters: next_filters,
            evaluation,
        }))
    }
}

#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, RowId};
    use rawscope_data::{
        LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, ScatterPointKind, ScatterPointRecord,
    };
    use rawscope_render::ScatterBrushSelection;

    use super::*;

    fn source() -> LoadedSourceTable {
        LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "winner".into(),
                kind: LoadedColumnKind::String,
            }],
            rows: ["white", "black"]
                .into_iter()
                .enumerate()
                .map(|(index, value)| LoadedSourceRow {
                    row_id: RowId(index as u64),
                    values: vec![value.into()],
                })
                .collect(),
        }
    }

    #[test]
    fn invalid_filter_keeps_last_valid_evaluation() {
        let source = source();
        let mut app = WorkbenchApp::default();
        app.scatter.source_rows = Some(source);
        app.initialize_scatter_filters();
        let valid = app.scatter_filters.evaluation.clone();

        app.apply_scatter_filter_action(FilterAction::SetNumericRange {
            column_name: "winner".into(),
            min_inclusive: 0.0,
            max_inclusive: 1.0,
            include_missing: false,
        });

        assert_eq!(app.scatter_filters.evaluation, valid);
        assert!(app.scatter_filters.error.is_some());
    }

    #[test]
    fn filter_change_clears_excluded_selection() {
        let mut app = WorkbenchApp::default();
        app.scatter.source_rows = Some(source());
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
        app.scatter.active_brush_selection = Some(ScatterBrushSelection {
            x_range: F32Range::new(0.0, 3.0),
            y_range: F32Range::new(0.0, 3.0),
        });
        app.initialize_scatter_filters();

        app.apply_scatter_filter_action(FilterAction::SetCategories {
            column_name: "winner".into(),
            included_values: vec!["white".into()],
            include_missing: false,
        });

        assert!(app.scatter.active_brush_selection.is_none());
        assert_eq!(
            app.scatter_filters
                .evaluation
                .as_ref()
                .unwrap()
                .included_count,
            1
        );
        assert!(app.render_schedule.is_refining());
    }

    #[test]
    fn filter_only_change_keeps_difference_baseline_clean() {
        let mut app = WorkbenchApp::default();
        app.scatter.source_rows = Some(source());
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
        app.scatter.difference_baseline_dirty = false;

        app.apply_scatter_filter_action(FilterAction::SetCategories {
            column_name: "winner".into(),
            included_values: vec!["white".into()],
            include_missing: false,
        });

        assert!(!app.scatter.difference_baseline_dirty);
    }
}
