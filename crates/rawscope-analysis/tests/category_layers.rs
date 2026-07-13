use std::num::NonZeroUsize;

use rawscope_analysis::{
    cohort::{CohortBuilder, CohortGenerationCounter, CohortPolicy, CohortSnapshot},
    visual_field::{CategoryLayerKind, CategoryLayerPlan},
};
use rawscope_core::{ColumnId, RowId};
use rawscope_data::{
    build_visual_field_catalog, CategoryMembershipIndex, ColumnChunk, DatasetChunk,
    DatasetGenerationCounter, DatasetIdentity, DatasetMemoryBudget, DatasetSchema,
    DatasetStoreBuilder, InvalidCell, InvalidCellReason, LoadedColumnKind, LoadedColumnSchema,
    LoadedSourceRow, LoadedSourceTable, NormalizedValue, SourceValue, StoreColumnKind, StoredCell,
    VisualFieldCatalogConfig,
};

fn fixture(
    values: &[Option<&str>],
    invalid_rows: &[usize],
    max_values: usize,
) -> (CategoryMembershipIndex, CohortSnapshot) {
    let schema = DatasetSchema::try_new([("category", StoreColumnKind::Utf8)]).unwrap();
    let cells = values
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            _ if invalid_rows.contains(&index) => StoredCell::invalid(
                None,
                InvalidCell::new(
                    RowId(index as u64),
                    ColumnId::new(0),
                    InvalidCellReason::ParseFailure,
                ),
            ),
            Some(value) => StoredCell::value(
                SourceValue::Utf8((*value).into()),
                NormalizedValue::Text((*value).into()),
            ),
            None => StoredCell::missing(None),
        })
        .collect::<Vec<_>>();
    let mut generations = DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(8, values.len()),
        schema,
        DatasetMemoryBudget::new(1_000_000),
        &mut generations,
    );
    builder
        .append_chunk(DatasetChunk::new(
            RowId(0),
            vec![ColumnChunk::new(ColumnId::new(0), cells)],
        ))
        .unwrap();
    let store = builder.finish();
    let source = LoadedSourceTable {
        columns: vec![LoadedColumnSchema {
            name: "category".into(),
            kind: LoadedColumnKind::String,
        }],
        rows: values
            .iter()
            .enumerate()
            .map(|(index, value)| LoadedSourceRow {
                row_id: RowId(index as u64),
                values: vec![value.unwrap_or("").to_string()],
            })
            .collect(),
    };
    let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
    let mut cohort_builder = CohortBuilder::new(CohortPolicy::default());
    let mut cohort_generations = CohortGenerationCounter::default();
    let cohort = cohort_builder
        .evaluate(
            &source,
            &catalog,
            store.generation(),
            &mut cohort_generations,
        )
        .unwrap();
    let index = store
        .category_index(ColumnId::new(0), NonZeroUsize::new(max_values).unwrap())
        .unwrap();
    (index, cohort)
}

#[test]
fn layer_plan_maps_unselected_values_to_other() {
    let (index, cohort) = fixture(&[Some("a"), Some("b"), Some("c")], &[], 2);
    let selected = [index.values()[0].id];
    let plan = CategoryLayerPlan::build(&index, &cohort, &selected).unwrap();
    assert!(plan.lookup().untracked_layer.is_some());
    assert!(plan
        .layers()
        .iter()
        .any(|layer| matches!(layer.kind, CategoryLayerKind::Other) && layer.row_count > 0));
}

#[test]
fn layer_plan_keeps_missing_and_invalid_distinct() {
    let (index, cohort) = fixture(&[Some("a"), None, None], &[2], 2);
    let plan = CategoryLayerPlan::build(&index, &cohort, &[]).unwrap();
    assert!(plan.lookup().missing_layer.is_some());
    assert!(plan.lookup().invalid_layer.is_some());
    assert!(plan
        .layers()
        .iter()
        .any(|layer| matches!(layer.kind, CategoryLayerKind::Missing)));
    assert!(plan
        .layers()
        .iter()
        .any(|layer| matches!(layer.kind, CategoryLayerKind::Invalid)));
}

#[test]
fn layer_plan_is_deterministic_under_ties() {
    let (index, cohort) = fixture(&[Some("b"), Some("a"), Some("b"), Some("a")], &[], 4);
    let first = CategoryLayerPlan::build(&index, &cohort, &[]).unwrap();
    let second = CategoryLayerPlan::build(&index, &cohort, &[]).unwrap();
    assert_eq!(first.layers(), second.layers());
    assert_eq!(first.layers()[0].label.as_ref(), "a");
}

#[test]
fn estimated_category_accuracy_is_not_promoted_to_exact() {
    let (index, cohort) = fixture(&[Some("a"), Some("b"), Some("c")], &[], 1);
    assert_ne!(
        index.accuracy(),
        rawscope_data::CategoryIndexAccuracy::Exact
    );
    let plan = CategoryLayerPlan::build(&index, &cohort, &[]).unwrap();
    assert!(plan
        .layers()
        .iter()
        .any(|layer| layer.accuracy != rawscope_data::CategoryIndexAccuracy::Exact));
}
