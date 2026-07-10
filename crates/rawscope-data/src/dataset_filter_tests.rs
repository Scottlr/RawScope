use rawscope_core::RowId;

use super::*;
use crate::{
    build_visual_field_catalog, LoadedColumnKind, LoadedColumnSchema, LoadedSourceRow,
    VisualFieldCatalogConfig,
};

fn source() -> LoadedSourceTable {
    LoadedSourceTable {
        columns: vec![
            LoadedColumnSchema {
                name: "rating".into(),
                kind: LoadedColumnKind::Integer,
            },
            LoadedColumnSchema {
                name: "winner".into(),
                kind: LoadedColumnKind::String,
            },
        ],
        rows: [
            ["1200", "white"],
            ["1800", "black"],
            ["", "white"],
            ["bad", ""],
        ]
        .into_iter()
        .enumerate()
        .map(|(index, values)| LoadedSourceRow {
            row_id: RowId(index as u64),
            values: values.into_iter().map(str::to_string).collect(),
        })
        .collect(),
    }
}

fn evaluate(filter_set: &FilterSet) -> FilterEvaluation {
    let source = source();
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    evaluate_filters(&source, &catalog, filter_set).unwrap()
}

#[test]
fn empty_filter_set_includes_every_row() {
    let evaluation = evaluate(&FilterSet::default());
    assert_eq!(evaluation.mask.as_gpu_u32_slice(), &[1, 1, 1, 1]);
    assert_eq!(
        (evaluation.included_count, evaluation.excluded_count),
        (4, 0)
    );
}

#[test]
fn all_included_mask_has_stable_gpu_alignment() {
    let mask = FilterMask::all_included(3);

    assert_eq!(mask.as_gpu_u32_slice(), &[1, 1, 1]);
    assert_eq!(mask.included_count(), 3);
}

#[test]
fn numeric_and_category_filters_compose_with_and() {
    let mut filters = FilterSet::default();
    filters.replace_for_column(DatasetFilter::NumericRange {
        column_name: "rating".into(),
        min_inclusive: 1000.0,
        max_inclusive: 1500.0,
        include_missing: false,
    });
    filters.replace_for_column(DatasetFilter::Categories {
        column_name: "winner".into(),
        included_values: vec!["white".into()],
        include_missing: false,
    });
    assert_eq!(evaluate(&filters).mask.as_gpu_u32_slice(), &[1, 0, 0, 0]);
}

#[test]
fn empty_category_set_excludes_non_missing_values() {
    let mut filters = FilterSet::default();
    filters.replace_for_column(DatasetFilter::Categories {
        column_name: "winner".into(),
        included_values: vec![],
        include_missing: false,
    });
    assert_eq!(evaluate(&filters).mask.as_gpu_u32_slice(), &[0, 0, 0, 0]);
}

#[test]
fn include_missing_is_explicit_per_predicate() {
    let mut filters = FilterSet::default();
    filters.replace_for_column(DatasetFilter::NumericRange {
        column_name: "rating".into(),
        min_inclusive: 1000.0,
        max_inclusive: 1300.0,
        include_missing: true,
    });
    assert_eq!(evaluate(&filters).mask.as_gpu_u32_slice(), &[1, 0, 1, 1]);
}

#[test]
fn filter_mask_preserves_row_id_alignment() {
    let mut source = source();
    source.rows[2].row_id = RowId(7);
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    let error = evaluate_filters(&source, &catalog, &FilterSet::default()).unwrap_err();
    assert!(matches!(
        error,
        FilterError::NonContiguousRowIds {
            expected: RowId(2),
            actual: RowId(7)
        }
    ));
}

#[test]
fn equal_replacement_does_not_increment_revision() {
    let mut filters = FilterSet::default();
    let filter = DatasetFilter::Categories {
        column_name: "winner".into(),
        included_values: vec!["white".into(), "black".into(), "white".into()],
        include_missing: false,
    };
    filters.replace_for_column(filter.clone());
    let revision = filters.revision;
    filters.replace_for_column(filter);
    assert_eq!(filters.revision, revision);
    assert_eq!(
        filters.filters[0],
        DatasetFilter::Categories {
            column_name: "winner".into(),
            included_values: vec!["black".into(), "white".into()],
            include_missing: false
        }
    );
}
