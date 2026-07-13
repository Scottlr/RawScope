use std::num::NonZeroUsize;

use rawscope_core::{ColumnId, RowId};
use rawscope_data::{
    CategoryCodeKind, CategoryIndexAccuracy, CategoryIndexValue, ColumnChunk, DatasetChunk,
    DatasetGenerationCounter, DatasetIdentity, DatasetMemoryBudget, DatasetSchema, DatasetStore,
    DatasetStoreBuilder, InvalidCell, InvalidCellReason, NormalizedValue, SourceValue,
    StoreColumnKind, StoredCell,
};

fn store(cells: Vec<StoredCell>, kind: StoreColumnKind) -> DatasetStore {
    let schema = DatasetSchema::try_new([("category", kind)]).unwrap();
    let row_count = cells.len();
    let mut generations = DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(7, row_count),
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
    builder.finish()
}

fn text(value: &str) -> StoredCell {
    StoredCell::value(
        SourceValue::Utf8(value.into()),
        NormalizedValue::Text(value.into()),
    )
}

#[test]
fn category_membership_aligns_with_dataset_rows() {
    let dataset = store(
        vec![
            text("a"),
            text("b"),
            StoredCell::missing(None),
            StoredCell::invalid(
                None,
                InvalidCell::new(RowId(3), ColumnId::new(0), InvalidCellReason::ParseFailure),
            ),
        ],
        StoreColumnKind::Utf8,
    );
    let index = dataset
        .category_index(ColumnId::new(0), NonZeroUsize::new(2).unwrap())
        .unwrap();
    assert_eq!(index.row_codes().len(), 4);
    assert!(matches!(
        index.code(RowId(0)),
        Some(CategoryCodeKind::Value(_))
    ));
    assert_eq!(index.code(RowId(2)), Some(CategoryCodeKind::Missing));
    assert_eq!(index.code(RowId(3)), Some(CategoryCodeKind::Invalid));
}

#[test]
fn high_cardinality_index_stays_within_value_budget() {
    let dataset = store(
        (0..100)
            .map(|index| text(&format!("value-{index}")))
            .collect(),
        StoreColumnKind::Utf8,
    );
    let index = dataset
        .category_index(ColumnId::new(0), NonZeroUsize::new(4).unwrap())
        .unwrap();
    assert!(index.values().len() <= 4);
    assert_eq!(index.row_codes().len(), 100);
    assert_eq!(index.accuracy(), CategoryIndexAccuracy::Truncated);
}

#[test]
fn typed_category_identity_is_not_merged_by_labels() {
    let dataset = store(
        vec![
            StoredCell::value(SourceValue::I64(1), NormalizedValue::I64(1)),
            StoredCell::value(SourceValue::I64(2), NormalizedValue::I64(2)),
        ],
        StoreColumnKind::I64,
    );
    let index = dataset
        .category_index(ColumnId::new(0), NonZeroUsize::new(4).unwrap())
        .unwrap();
    assert_eq!(index.values()[0].label.as_ref(), "1");
    assert_eq!(index.values()[0].value, CategoryIndexValue::I64(1));
}
