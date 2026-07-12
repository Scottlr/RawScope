//! Stable-ordinal schema diffs with duplicate-name ambiguity.

use std::collections::BTreeSet;

use rawscope_data::{LoadedColumnKind, LoadedColumnSchema};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaDiffStatus {
    Unchanged,
    Added,
    Removed,
    TypeChanged,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDiffColumn {
    pub ordinal: usize,
    pub before_name: Option<String>,
    pub after_name: Option<String>,
    pub before_kind: Option<LoadedColumnKind>,
    pub after_kind: Option<LoadedColumnKind>,
    pub status: SchemaDiffStatus,
}

pub fn diff_schemas(
    before: &[LoadedColumnSchema],
    after: &[LoadedColumnSchema],
) -> Vec<SchemaDiffColumn> {
    let max_len = before.len().max(after.len());
    let before_duplicates = duplicate_names(before);
    let after_duplicates = duplicate_names(after);
    (0..max_len)
        .map(|ordinal| {
            let previous = before.get(ordinal);
            let current = after.get(ordinal);
            let status = if previous.is_some_and(|column| before_duplicates.contains(&column.name))
                || current.is_some_and(|column| after_duplicates.contains(&column.name))
            {
                SchemaDiffStatus::Ambiguous
            } else {
                match (previous, current) {
                    (Some(previous), Some(current)) if previous.kind == current.kind => {
                        SchemaDiffStatus::Unchanged
                    }
                    (Some(_), Some(_)) => SchemaDiffStatus::TypeChanged,
                    (None, Some(_)) => SchemaDiffStatus::Added,
                    (Some(_), None) => SchemaDiffStatus::Removed,
                    (None, None) => unreachable!("ordinal is within the maximum schema length"),
                }
            };
            SchemaDiffColumn {
                ordinal,
                before_name: previous.map(|column| column.name.clone()),
                after_name: current.map(|column| column.name.clone()),
                before_kind: previous.map(|column| column.kind),
                after_kind: current.map(|column| column.kind),
                status,
            }
        })
        .collect()
}

fn duplicate_names(columns: &[LoadedColumnSchema]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for column in columns {
        if !seen.insert(column.name.clone()) {
            duplicates.insert(column.name.clone());
        }
    }
    duplicates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_diff_uses_ordinal_and_surfaces_duplicate_ambiguity() {
        let before = vec![LoadedColumnSchema {
            name: "score".to_string(),
            kind: LoadedColumnKind::Integer,
        }];
        let after = vec![
            LoadedColumnSchema {
                name: "score".to_string(),
                kind: LoadedColumnKind::Float,
            },
            LoadedColumnSchema {
                name: "score".to_string(),
                kind: LoadedColumnKind::Float,
            },
        ];
        let diff = diff_schemas(&before, &after);
        assert_eq!(diff[0].status, SchemaDiffStatus::Ambiguous);
        assert_eq!(diff[1].status, SchemaDiffStatus::Ambiguous);
    }
}
