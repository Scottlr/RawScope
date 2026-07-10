//! Bounded summaries for comparing two local source tables.

use std::collections::{BTreeMap, BTreeSet};

use rawscope_data::{LoadedColumnKind, LoadedSourceTable};

/// Column-level diff status for one shared or changed schema entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetDiffColumnStatus {
    Unchanged,
    Added,
    Removed,
    TypeChanged,
}

/// One schema entry in a bounded dataset diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetDiffColumn {
    pub name: String,
    pub status: DatasetDiffColumnStatus,
    pub before_type: Option<LoadedColumnKind>,
    pub after_type: Option<LoadedColumnKind>,
}

/// Missingness change for one shared column.
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetDiffMissingnessDelta {
    pub column_name: String,
    pub before_missing_count: u64,
    pub after_missing_count: u64,
    pub delta_missing_count: i64,
    pub before_missing_ratio: f32,
    pub after_missing_ratio: f32,
    pub delta_percentage_points: f32,
}

/// Compact dataset diff summary for two loaded local source tables.
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetDiffSummary {
    pub before_row_count: u64,
    pub after_row_count: u64,
    pub row_count_delta: i64,
    pub columns: Vec<DatasetDiffColumn>,
    pub missingness: Vec<DatasetDiffMissingnessDelta>,
}

/// Builds a bounded diff summary for two retained local source tables.
pub fn dataset_diff_summary(
    before: &LoadedSourceTable,
    after: &LoadedSourceTable,
) -> DatasetDiffSummary {
    let before_row_count = before.rows.len() as u64;
    let after_row_count = after.rows.len() as u64;
    let before_columns = column_kinds_by_name(before);
    let after_columns = column_kinds_by_name(after);
    let all_column_names = before_columns
        .keys()
        .chain(after_columns.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let columns = all_column_names
        .into_iter()
        .map(|column_name| {
            let before_type = before_columns.get(column_name).copied().copied();
            let after_type = after_columns.get(column_name).copied().copied();
            let status = match (before_type, after_type) {
                (Some(before_type), Some(after_type)) if before_type == after_type => {
                    DatasetDiffColumnStatus::Unchanged
                }
                (Some(_), Some(_)) => DatasetDiffColumnStatus::TypeChanged,
                (None, Some(_)) => DatasetDiffColumnStatus::Added,
                (Some(_), None) => DatasetDiffColumnStatus::Removed,
                (None, None) => DatasetDiffColumnStatus::Unchanged,
            };

            DatasetDiffColumn {
                name: column_name.to_string(),
                status,
                before_type,
                after_type,
            }
        })
        .collect();

    let before_missingness = missingness_counts_by_name(before);
    let after_missingness = missingness_counts_by_name(after);
    let missingness = before_missingness
        .keys()
        .filter(|column_name| after_missingness.contains_key(*column_name))
        .map(|column_name| {
            let before_missing_count = before_missingness.get(column_name).copied().unwrap_or(0);
            let after_missing_count = after_missingness.get(column_name).copied().unwrap_or(0);
            let before_missing_ratio = ratio_u64(before_missing_count, before_row_count);
            let after_missing_ratio = ratio_u64(after_missing_count, after_row_count);

            DatasetDiffMissingnessDelta {
                column_name: (*column_name).to_string(),
                before_missing_count,
                after_missing_count,
                delta_missing_count: delta_i64(after_missing_count, before_missing_count),
                before_missing_ratio,
                after_missing_ratio,
                delta_percentage_points: (after_missing_ratio - before_missing_ratio) * 100.0,
            }
        })
        .collect();

    DatasetDiffSummary {
        before_row_count,
        after_row_count,
        row_count_delta: delta_i64(after_row_count, before_row_count),
        columns,
        missingness,
    }
}

fn column_kinds_by_name(table: &LoadedSourceTable) -> BTreeMap<&str, &LoadedColumnKind> {
    table
        .columns
        .iter()
        .map(|column| (column.name.as_str(), &column.kind))
        .collect()
}

fn missingness_counts_by_name(table: &LoadedSourceTable) -> BTreeMap<&str, u64> {
    let mut missing_counts = vec![0_u64; table.columns.len()];

    for row in &table.rows {
        for (column_index, count) in missing_counts.iter_mut().enumerate() {
            let cell_value = row.values.get(column_index).map(String::as_str);
            if cell_value.is_some_and(is_missing_value) {
                *count += 1;
            }
        }
    }

    table
        .columns
        .iter()
        .zip(missing_counts)
        .map(|(column, missing_count)| (column.name.as_str(), missing_count))
        .collect()
}

fn ratio_u64(count: u64, total_count: u64) -> f32 {
    if total_count == 0 {
        return 0.0;
    }

    count as f32 / total_count as f32
}

fn delta_i64(after: u64, before: u64) -> i64 {
    let delta = after as i128 - before as i128;
    delta.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn is_missing_value(value: &str) -> bool {
    value.trim().is_empty()
}
