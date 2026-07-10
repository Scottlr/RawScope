//! Deterministic visual summaries derived from retained source rows.

use std::collections::BTreeMap;

use crate::{LoadedColumnKind, LoadedSourceTable};

#[derive(Debug, Clone, PartialEq)]
pub enum VisualFieldSummary {
    Numeric(NumericFieldSummary),
    Categorical(CategoricalFieldSummary),
    Empty { missing_count: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericFieldSummary {
    pub min: f64,
    pub max: f64,
    pub valid_count: usize,
    pub missing_count: usize,
    pub invalid_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryValueCount {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoricalFieldSummary {
    pub values: Vec<CategoryValueCount>,
    pub distinct_count: usize,
    pub missing_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualFieldDescriptor {
    pub column_name: String,
    pub column_index: usize,
    pub source_kind: LoadedColumnKind,
    pub summary: VisualFieldSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualFieldCatalog {
    pub row_count: usize,
    pub fields: Vec<VisualFieldDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualFieldCatalogConfig {
    pub max_category_values: usize,
}

impl Default for VisualFieldCatalogConfig {
    fn default() -> Self {
        Self {
            max_category_values: 4_096,
        }
    }
}

pub fn build_visual_field_catalog(
    source: &LoadedSourceTable,
    config: VisualFieldCatalogConfig,
) -> VisualFieldCatalog {
    let fields = source
        .columns
        .iter()
        .enumerate()
        .map(|(column_index, column)| VisualFieldDescriptor {
            column_name: column.name.clone(),
            column_index,
            source_kind: column.kind,
            summary: summarize_column(source, column_index, column.kind, config),
        })
        .collect();
    VisualFieldCatalog {
        row_count: source.rows.len(),
        fields,
    }
}

fn summarize_column(
    source: &LoadedSourceTable,
    column_index: usize,
    kind: LoadedColumnKind,
    config: VisualFieldCatalogConfig,
) -> VisualFieldSummary {
    match kind {
        LoadedColumnKind::Integer | LoadedColumnKind::Float => {
            VisualFieldSummary::Numeric(summarize_numeric(source, column_index))
        }
        LoadedColumnKind::String => VisualFieldSummary::Categorical(summarize_categorical(
            source,
            column_index,
            config.max_category_values,
        )),
        LoadedColumnKind::Empty => VisualFieldSummary::Empty {
            missing_count: source.rows.len(),
        },
    }
}

fn summarize_numeric(source: &LoadedSourceTable, column_index: usize) -> NumericFieldSummary {
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    let mut valid_count = 0;
    let mut missing_count = 0;
    let mut invalid_count = 0;
    for row in &source.rows {
        let value = row
            .values
            .get(column_index)
            .map_or("", String::as_str)
            .trim();
        if value.is_empty() {
            missing_count += 1;
        } else if let Ok(parsed) = value.parse::<f64>() {
            if parsed.is_finite() {
                min = Some(min.map_or(parsed, |current| current.min(parsed)));
                max = Some(max.map_or(parsed, |current| current.max(parsed)));
                valid_count += 1;
            } else {
                invalid_count += 1;
            }
        } else {
            invalid_count += 1;
        }
    }
    NumericFieldSummary {
        min: min.unwrap_or(0.0),
        max: max.unwrap_or(0.0),
        valid_count,
        missing_count,
        invalid_count,
    }
}

fn summarize_categorical(
    source: &LoadedSourceTable,
    column_index: usize,
    max_category_values: usize,
) -> CategoricalFieldSummary {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut missing_count = 0;
    for row in &source.rows {
        let value = row
            .values
            .get(column_index)
            .map_or("", String::as_str)
            .trim();
        if value.is_empty() {
            missing_count += 1;
        } else {
            *counts.entry(value.to_string()).or_default() += 1;
        }
    }
    let distinct_count = counts.len();
    let mut values = counts
        .into_iter()
        .map(|(value, count)| CategoryValueCount { value, count })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    values.truncate(max_category_values);
    CategoricalFieldSummary {
        values,
        distinct_count,
        missing_count,
        truncated: distinct_count > max_category_values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoadedColumnSchema, LoadedSourceRow};
    use rawscope_core::RowId;

    fn table(kind: LoadedColumnKind, values: &[&str]) -> LoadedSourceTable {
        LoadedSourceTable {
            columns: vec![LoadedColumnSchema {
                name: "field".into(),
                kind,
            }],
            rows: values
                .iter()
                .enumerate()
                .map(|(index, value)| LoadedSourceRow {
                    row_id: RowId(index as u64),
                    values: vec![(*value).into()],
                })
                .collect(),
        }
    }

    #[test]
    fn numeric_catalog_reports_range_missing_and_invalid_counts() {
        let catalog = build_visual_field_catalog(
            &table(LoadedColumnKind::Float, &[" 2.5 ", "", "bad", "-1"]),
            VisualFieldCatalogConfig::default(),
        );
        let VisualFieldSummary::Numeric(summary) = &catalog.fields[0].summary else {
            panic!("expected numeric summary")
        };
        assert_eq!((summary.min, summary.max), (-1.0, 2.5));
        assert_eq!(
            (
                summary.valid_count,
                summary.missing_count,
                summary.invalid_count
            ),
            (2, 1, 1)
        );
    }

    #[test]
    fn categorical_catalog_orders_by_count_then_value() {
        let catalog = build_visual_field_catalog(
            &table(
                LoadedColumnKind::String,
                &["beta", "alpha", "beta", "alpha", "gamma"],
            ),
            VisualFieldCatalogConfig::default(),
        );
        let VisualFieldSummary::Categorical(summary) = &catalog.fields[0].summary else {
            panic!("expected categorical summary")
        };
        assert_eq!(
            summary
                .values
                .iter()
                .map(|value| value.value.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn categorical_catalog_discloses_truncation() {
        let catalog = build_visual_field_catalog(
            &table(LoadedColumnKind::String, &["a", "b", "c"]),
            VisualFieldCatalogConfig {
                max_category_values: 2,
            },
        );
        let VisualFieldSummary::Categorical(summary) = &catalog.fields[0].summary else {
            panic!("expected categorical summary")
        };
        assert_eq!(summary.distinct_count, 3);
        assert_eq!(summary.values.len(), 2);
        assert!(summary.truncated);
    }
}
