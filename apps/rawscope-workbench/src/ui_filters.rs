//! Bounded UI projection and controls for the active scatter cohort.

use egui::{CollapsingHeader, RichText, ScrollArea, Slider, Ui};
use rawscope_data::{DatasetFilter, VisualFieldSummary};

use crate::{app::WorkbenchApp, ui_theme::TEXT_MUTED};

const MAX_VISIBLE_CATEGORY_OPTIONS: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FilterControlState {
    NumericRange {
        column_name: String,
        domain_min: f64,
        domain_max: f64,
        selected_min: f64,
        selected_max: f64,
        include_missing: bool,
    },
    Categories {
        column_name: String,
        values: Vec<CategoryOptionState>,
        distinct_count: usize,
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
    let evaluation = app.scatter_filters.evaluation.as_ref()?;
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
        included_count: evaluation.included_count,
        total_count: catalog.row_count,
        error: app.scatter_filters.error.clone(),
    })
}

pub(crate) fn show_filters(
    ui: &mut Ui,
    state: Option<&ScatterFiltersUiState>,
) -> Option<FilterAction> {
    let state = state?;
    let mut action = None;
    ui.horizontal(|ui| {
        ui.heading("Filters");
        ui.label(
            RichText::new(format!(
                "{} / {} rows",
                state.included_count, state.total_count
            ))
            .small()
            .color(TEXT_MUTED),
        );
    });
    if !state.active_columns.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for column in &state.active_columns {
                ui.label(RichText::new(column).small());
                if ui.small_button("Clear").clicked() {
                    action = Some(FilterAction::RemoveColumn {
                        column_name: column.clone(),
                    });
                }
            }
            if ui.small_button("Clear all").clicked() {
                action = Some(FilterAction::ClearAll);
            }
        });
    }
    if let Some(error) = &state.error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
    for control in &state.controls {
        CollapsingHeader::new(control.column_name())
            .default_open(true)
            .show(ui, |ui| {
                let next = match control {
                    FilterControlState::NumericRange {
                        column_name,
                        domain_min,
                        domain_max,
                        selected_min,
                        selected_max,
                        include_missing,
                    } => show_numeric_filter(
                        ui,
                        column_name,
                        *domain_min,
                        *domain_max,
                        *selected_min,
                        *selected_max,
                        *include_missing,
                    ),
                    FilterControlState::Categories {
                        column_name,
                        values,
                        distinct_count,
                        truncated,
                        include_missing,
                    } => show_category_filter(
                        ui,
                        column_name,
                        values,
                        *distinct_count,
                        *truncated,
                        *include_missing,
                    ),
                };
                if next.is_some() {
                    action = next;
                }
            });
    }
    ui.menu_button("Add filter", |ui| {
        for column in &state.available_columns {
            if ui.button(column).clicked() {
                action = Some(FilterAction::AddColumn {
                    column_name: column.clone(),
                });
                ui.close();
            }
        }
    });
    action
}

fn show_numeric_filter(
    ui: &mut Ui,
    column_name: &str,
    domain_min: f64,
    domain_max: f64,
    mut selected_min: f64,
    mut selected_max: f64,
    mut include_missing: bool,
) -> Option<FilterAction> {
    let min_changed = ui
        .add(Slider::new(&mut selected_min, domain_min..=domain_max).text("Minimum"))
        .changed();
    let max_changed = ui
        .add(Slider::new(&mut selected_max, domain_min..=domain_max).text("Maximum"))
        .changed();
    let missing_changed = ui
        .checkbox(&mut include_missing, "Include missing")
        .changed();
    (min_changed || max_changed || missing_changed).then(|| FilterAction::SetNumericRange {
        column_name: column_name.to_string(),
        min_inclusive: selected_min.min(selected_max),
        max_inclusive: selected_min.max(selected_max),
        include_missing,
    })
}

fn show_category_filter(
    ui: &mut Ui,
    column_name: &str,
    values: &[CategoryOptionState],
    distinct_count: usize,
    truncated: bool,
    mut include_missing: bool,
) -> Option<FilterAction> {
    let search_id = ui.id().with(("filter-search", column_name));
    let mut search = ui
        .data(|data| data.get_temp::<String>(search_id))
        .unwrap_or_default();
    let search_changed = ui.text_edit_singleline(&mut search).changed();
    if search_changed {
        ui.data_mut(|data| data.insert_temp(search_id, search.clone()));
    }
    let normalized_search = search.trim().to_lowercase();
    let mut selected = values
        .iter()
        .filter(|value| value.selected)
        .map(|value| value.value.clone())
        .collect::<Vec<_>>();
    let mut changed = false;
    ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
        for value in values
            .iter()
            .filter(|value| {
                normalized_search.is_empty()
                    || value.value.to_lowercase().contains(&normalized_search)
            })
            .take(MAX_VISIBLE_CATEGORY_OPTIONS)
        {
            let mut is_selected = value.selected;
            if ui
                .checkbox(
                    &mut is_selected,
                    format!("{} ({})", value.value, value.count),
                )
                .changed()
            {
                changed = true;
                if is_selected {
                    selected.push(value.value.clone());
                } else {
                    selected.retain(|candidate| candidate != &value.value);
                }
            }
        }
    });
    ui.label(
        RichText::new(if truncated {
            format!(
                "Showing up to {MAX_VISIBLE_CATEGORY_OPTIONS} matches from {} retained / {distinct_count} distinct",
                values.len()
            )
        } else {
            format!(
                "Showing up to {MAX_VISIBLE_CATEGORY_OPTIONS} matches / {distinct_count} distinct"
            )
        })
        .small()
        .color(TEXT_MUTED),
    );
    changed |= ui
        .checkbox(&mut include_missing, "Include missing")
        .changed();
    changed.then(|| FilterAction::SetCategories {
        column_name: column_name.to_string(),
        included_values: selected,
        include_missing,
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
