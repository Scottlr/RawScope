//! Bounded UI projection and controls for the active scatter cohort.

mod category_control;
mod controls;

use rawscope_data::{DatasetFilter, VisualFieldSummary};

use crate::app::WorkbenchApp;

pub(crate) use controls::show_filters;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FilterControlState {
    NumericRange {
        column_name: String,
        domain_min: f64,
        domain_max: f64,
        selected_min: f64,
        selected_max: f64,
        missing_count: usize,
        invalid_count: usize,
        include_missing: bool,
    },
    Categories {
        column_name: String,
        values: Vec<CategoryOptionState>,
        distinct_count: usize,
        missing_count: usize,
        truncated: bool,
        include_missing: bool,
    },
}

impl FilterControlState {
    fn column_name(&self) -> &str {
        match self {
            Self::NumericRange { column_name, .. } | Self::Categories { column_name, .. } => {
                column_name
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryOptionState {
    pub(crate) value: String,
    pub(crate) count: usize,
    pub(crate) selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScatterFiltersUiState {
    pub(crate) controls: Vec<FilterControlState>,
    pub(crate) available_columns: Vec<String>,
    pub(crate) active_columns: Vec<String>,
    pub(crate) included_count: usize,
    pub(crate) total_count: usize,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FilterAction {
    SetNumericRange {
        column_name: String,
        min_inclusive: f64,
        max_inclusive: f64,
        include_missing: bool,
    },
    SetCategories {
        column_name: String,
        included_values: Vec<String>,
        include_missing: bool,
    },
    RemoveColumn {
        column_name: String,
    },
    ClearAll,
    AddColumn {
        column_name: String,
    },
}

pub(crate) fn scatter_filters_ui_state(app: &WorkbenchApp) -> Option<ScatterFiltersUiState> {
    let catalog = app.scatter_filters.catalog.as_ref()?;
    let controls = app
        .scatter_filters
        .visible_columns
        .iter()
        .filter_map(|column_name| {
            let field = catalog
                .fields
                .iter()
                .find(|field| field.column_name == *column_name)?;
            let active = app
                .scatter_filters
                .filters
                .filters
                .iter()
                .find(|filter| filter.column_name() == column_name);
            match &field.summary {
                VisualFieldSummary::Numeric(summary) => {
                    let (selected_min, selected_max, include_missing) = match active {
                        Some(DatasetFilter::NumericRange {
                            min_inclusive,
                            max_inclusive,
                            include_missing,
                            ..
                        }) => (*min_inclusive, *max_inclusive, *include_missing),
                        _ => (summary.min, summary.max, false),
                    };
                    Some(FilterControlState::NumericRange {
                        column_name: column_name.clone(),
                        domain_min: summary.min,
                        domain_max: summary.max,
                        selected_min,
                        selected_max,
                        missing_count: summary.missing_count,
                        invalid_count: summary.invalid_count,
                        include_missing,
                    })
                }
                VisualFieldSummary::Categorical(summary) => {
                    let (selected_values, include_missing) = match active {
                        Some(DatasetFilter::Categories {
                            included_values,
                            include_missing,
                            ..
                        }) => (Some(included_values.as_slice()), *include_missing),
                        _ => (None, false),
                    };
                    Some(FilterControlState::Categories {
                        column_name: column_name.clone(),
                        values: summary
                            .values
                            .iter()
                            .map(|value| CategoryOptionState {
                                value: value.value.clone(),
                                count: value.count,
                                selected: selected_values.is_none_or(|selected| {
                                    selected.iter().any(|candidate| candidate == &value.value)
                                }),
                            })
                            .collect(),
                        distinct_count: summary.distinct_count,
                        missing_count: summary.missing_count,
                        truncated: summary.truncated,
                        include_missing,
                    })
                }
                VisualFieldSummary::Empty { .. } => None,
            }
        })
        .collect();
    let available_columns = catalog
        .fields
        .iter()
        .filter(|field| {
            !app.scatter_filters
                .visible_columns
                .contains(&field.column_name)
                && !matches!(field.summary, VisualFieldSummary::Empty { .. })
        })
        .map(|field| field.column_name.clone())
        .collect();
    Some(ScatterFiltersUiState {
        controls,
        available_columns,
        active_columns: app
            .scatter_filters
            .filters
            .filters
            .iter()
            .map(|filter| filter.column_name().to_string())
            .collect(),
        included_count: app.scatter_filters.cohort_snapshot.as_ref().map_or_else(
            || {
                app.scatter_filters
                    .evaluation
                    .as_ref()
                    .map_or(catalog.row_count, |evaluation| evaluation.included_count)
            },
            |snapshot| snapshot.included_row_count() as usize,
        ),
        total_count: catalog.row_count,
        error: app.scatter_filters.error.clone(),
    })
}

#[cfg(test)]
mod tests {
    use rawscope_core::RowId;
    use rawscope_data::{
        DatasetProfileId, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable,
    };

    use crate::app_scatter_filter::ScatterFilterState;

    #[test]
    fn lichess_filter_order_matches_profile_hints() {
        let source = LoadedSourceTable {
            columns: [
                "opening",
                "winner",
                "termination",
                "category",
                "time_control",
            ]
            .into_iter()
            .map(|name| LoadedColumnSchema {
                name: name.into(),
                kind: LoadedColumnKind::String,
            })
            .collect(),
            rows: vec![LoadedSourceRow {
                row_id: RowId(0),
                values: vec![
                    "x".into(),
                    "white".into(),
                    "normal".into(),
                    "blitz".into(),
                    "300+0".into(),
                ],
            }],
        };
        let mut filters = ScatterFilterState::default();
        filters.initialize(&source, Some(DatasetProfileId::LichessGames));

        assert_eq!(
            filters.visible_columns,
            ["winner", "category", "time_control", "termination"]
        );
    }
}
