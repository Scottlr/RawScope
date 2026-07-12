//! Scatter filter catalog, evaluation, and cohort revision coordination.

use rawscope_analysis::cohort::{
    CohortBuilder, CohortGenerationCounter, CohortPolicy, CohortSnapshot,
};
use rawscope_data::DatasetGeneration;
use rawscope_data::{
    available_profile_filter_hints, build_visual_field_catalog, dataset_profile, evaluate_filters,
    DatasetFilter, DatasetProfileId, FilterEvaluation, FilterRevision, FilterSet,
    LoadedSourceTable, VisualFieldCatalog, VisualFieldCatalogConfig,
};

use crate::{app::WorkbenchApp, ui::ExportStatus, ui_filters::FilterAction};

const DEFAULT_EXPANDED_FILTER_COUNT: usize = 4;

#[derive(Debug, Clone)]
pub(crate) struct ScatterFilterState {
    pub(crate) catalog: Option<VisualFieldCatalog>,
    pub(crate) filters: FilterSet,
    pub(crate) evaluation: Option<FilterEvaluation>,
    pub(crate) last_uploaded_revision: FilterRevision,
    pub(crate) visible_columns: Vec<String>,
    pub(crate) error: Option<String>,
    pub(crate) cohort_snapshot: Option<CohortSnapshot>,
    pub(crate) cohort_builder: CohortBuilder,
    pub(crate) cohort_generations: CohortGenerationCounter,
    pub(crate) dataset_generation: DatasetGeneration,
}

impl Default for ScatterFilterState {
    fn default() -> Self {
        Self {
            catalog: None,
            filters: FilterSet::default(),
            evaluation: None,
            last_uploaded_revision: FilterRevision::default(),
            visible_columns: Vec::new(),
            error: None,
            cohort_snapshot: None,
            cohort_builder: CohortBuilder::new(CohortPolicy::default()),
            cohort_generations: CohortGenerationCounter::default(),
            dataset_generation: rawscope_data::DatasetGenerationCounter::default().mint(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedFilterState {
    pub(crate) filters: FilterSet,
    pub(crate) evaluation: FilterEvaluation,
    pub(crate) cohort_snapshot: CohortSnapshot,
    pub(crate) cohort_builder: CohortBuilder,
    pub(crate) cohort_generations: CohortGenerationCounter,
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
        let mut cohort_builder = CohortBuilder::new(CohortPolicy::default());
        let mut cohort_generations = CohortGenerationCounter::default();
        let dataset_generation = rawscope_data::DatasetGenerationCounter::default().mint();
        let cohort_snapshot = cohort_builder
            .evaluate(
                source,
                &catalog,
                dataset_generation,
                &mut cohort_generations,
            )
            .ok();
        *self = Self {
            catalog: Some(catalog),
            filters,
            evaluation,
            last_uploaded_revision: FilterRevision::default(),
            visible_columns,
            error: None,
            cohort_snapshot,
            cohort_builder,
            cohort_generations,
            dataset_generation,
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
            .initialize(source, self.workbench_state.active_dataset_profile);
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
        let cohort_mask = prepared.cohort_snapshot.filter_mask();
        let cohort_revision = prepared.cohort_snapshot.filter_revision();
        let cohort_included_count = prepared.cohort_snapshot.included_row_count();
        let evaluation = prepared.evaluation;
        let difference_remains_available = next_filters.is_active() && cohort_included_count > 0;

        if let Some(renderer) = self.scatter.density_renderer.as_ref() {
            if let Err(err) = renderer.validate_filter_mask(&cohort_mask) {
                self.scatter_filters.error = Some(err.to_string());
                self.request_redraw();
                return;
            }
        }
        if let Some(renderer) = self.scatter.difference_renderer.as_ref() {
            if let Err(err) = renderer.validate_filter_mask(&cohort_mask) {
                self.scatter_filters.error = Some(err.to_string());
                self.request_redraw();
                return;
            }
        }

        let upload_result = self
            .gpu
            .as_ref()
            .zip(self.scatter.density_renderer.as_mut())
            .map(|(gpu, renderer)| {
                renderer.update_filter_mask(gpu.queue(), &cohort_mask, cohort_revision)
            });
        if let Err(err) = upload_result.transpose() {
            self.scatter_filters.error = Some(err.to_string());
            return;
        }
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.difference_renderer.as_mut())
        {
            if let Err(err) =
                renderer.update_filter_mask(gpu.queue(), &cohort_mask, cohort_revision)
            {
                self.scatter_filters.error = Some(err.to_string());
                return;
            }
        }

        self.scatter_filters.last_uploaded_revision = cohort_revision;
        self.scatter_filters.filters = next_filters;
        self.scatter_filters.evaluation = Some(evaluation);
        self.scatter_filters.cohort_snapshot = Some(prepared.cohort_snapshot);
        self.scatter_filters.cohort_builder = prepared.cohort_builder;
        self.scatter_filters.cohort_generations = prepared.cohort_generations;
        self.scatter_filters.error = None;
        if !difference_remains_available {
            self.scatter.density_mode = rawscope_render::ScatterDensityMode::AbsoluteDensity;
        }
        self.invalidate_scatter_inspection();
        self.invalidate_scatter_point_reveal();
        self.clear_brush();
        self.workbench_state.export_status = ExportStatus::Idle;
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
        match action.clone() {
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
        let mut cohort_builder = self.scatter_filters.cohort_builder.clone();
        let mut cohort_generations = self.scatter_filters.cohort_generations;
        match action {
            FilterAction::SetNumericRange {
                column_name,
                min_inclusive,
                max_inclusive,
                include_missing,
            } => cohort_builder.replace_filter(DatasetFilter::NumericRange {
                column_name,
                min_inclusive,
                max_inclusive,
                include_missing,
            }),
            FilterAction::SetCategories {
                column_name,
                included_values,
                include_missing,
            } => cohort_builder.replace_filter(DatasetFilter::Categories {
                column_name,
                included_values,
                include_missing,
            }),
            FilterAction::RemoveColumn { column_name } => {
                cohort_builder.remove_column(&column_name);
            }
            FilterAction::ClearAll => cohort_builder.clear(),
            FilterAction::AddColumn { .. } => unreachable!("handled above"),
        }
        let cohort_snapshot = cohort_builder
            .evaluate(
                source,
                catalog,
                self.scatter_filters.dataset_generation,
                &mut cohort_generations,
            )
            .map_err(|err| err.to_string())?;
        Ok(Some(PreparedFilterState {
            filters: next_filters,
            evaluation,
            cohort_snapshot,
            cohort_builder,
            cohort_generations,
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
        let cohort = app
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .expect("successful filter changes publish a cohort snapshot");
        assert_eq!(cohort.included_row_ids(), &[RowId(0)]);
        assert_eq!(cohort.included_row_count(), 1);
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
