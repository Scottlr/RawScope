use std::sync::Arc;

use rawscope_core::{ColumnId, RowId};
use rawscope_data::{
    CellRef, CellState, ColumnChunk, DatasetChunk, DatasetEvidenceKey, DatasetIdentity,
    DatasetMemoryBudget, DatasetSchema, DatasetStoreBuilder, DecodedCsvCell, InvalidCell,
    InvalidCellReason, NormalizedValue, SourceValue, StoreColumnKind, StoredCell,
};

fn schema() -> DatasetSchema {
    DatasetSchema::try_new([
        ("name", StoreColumnKind::Utf8),
        ("score", StoreColumnKind::F64),
    ])
    .expect("fixture schema is valid")
}

#[test]
fn decoded_csv_cell_preserves_raw_spelling_separately_from_normalized_state() {
    let cell = DecodedCsvCell::new(
        Arc::<str>::from("  0042 "),
        CellState::Value(NormalizedValue::I64(42)),
    );
    assert_eq!(cell.raw(), "  0042 ");
    assert!(matches!(
        cell.analytical(),
        CellState::Value(NormalizedValue::I64(42))
    ));
}

fn chunk(row_id_start: u64, score: f64) -> DatasetChunk {
    DatasetChunk::new(
        RowId(row_id_start),
        vec![
            ColumnChunk::new(
                ColumnId::new(0),
                vec![StoredCell::value(
                    SourceValue::Utf8(Arc::from("  alice ")),
                    NormalizedValue::Text(Arc::from("alice")),
                )],
            ),
            ColumnChunk::new(
                ColumnId::new(1),
                vec![StoredCell::value(
                    SourceValue::F64(score),
                    NormalizedValue::F64(score),
                )],
            ),
        ],
    )
}

#[test]
fn store_rejects_duplicate_schema_names() {
    let error = DatasetSchema::try_new([
        ("score", StoreColumnKind::F64),
        ("score", StoreColumnKind::F64),
    ])
    .expect_err("duplicate names must be rejected");
    assert!(matches!(
        error,
        rawscope_data::DatasetSchemaError::DuplicateName { .. }
    ));
}

#[test]
fn failed_append_does_not_mutate_store_builder() {
    let mut generations = rawscope_data::DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(1, 2),
        schema(),
        DatasetMemoryBudget::new(10_000),
        &mut generations,
    );
    builder
        .append_chunk(chunk(0, 1.0))
        .expect("first chunk fits");
    let error = builder
        .append_chunk(chunk(3, 2.0))
        .expect_err("row ids must remain contiguous");
    assert!(matches!(
        error,
        rawscope_data::DatasetStoreError::Chunk(
            rawscope_data::ChunkValidationError::NonContiguousRows { .. }
        )
    ));
    let store = builder.finish();
    assert_eq!(store.row_count(), 1);
}

#[test]
fn store_preserves_raw_values_and_distinguishes_invalid_cells() {
    let mut generations = rawscope_data::DatasetGenerationCounter::default();
    let schema = DatasetSchema::try_new([("score", StoreColumnKind::F64)]).unwrap();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(1, 1),
        schema,
        DatasetMemoryBudget::new(10_000),
        &mut generations,
    );
    let invalid = InvalidCell::new(RowId(0), ColumnId::new(0), InvalidCellReason::ParseFailure);
    builder
        .append_chunk(DatasetChunk::new(
            RowId(0),
            vec![ColumnChunk::new(
                ColumnId::new(0),
                vec![StoredCell::invalid(
                    Some(SourceValue::Utf8(Arc::from("not-a-number"))),
                    invalid,
                )],
            )],
        ))
        .unwrap();
    let store = builder.finish();
    assert!(matches!(
        store.source_value(RowId(0), ColumnId::new(0)).unwrap(),
        CellRef::Invalid(cell) if *cell == invalid
    ));
}

#[test]
fn budget_failure_happens_before_append() {
    let mut generations = rawscope_data::DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(1, 1),
        schema(),
        DatasetMemoryBudget::new(1),
        &mut generations,
    );
    let error = builder
        .append_chunk(chunk(0, 1.0))
        .expect_err("chunk exceeds budget");
    assert!(matches!(error, rawscope_data::DatasetStoreError::Budget(_)));
    assert_eq!(builder.finish().row_count(), 0);
}

#[test]
fn evidence_key_binds_to_store_generation_and_column_id() {
    let mut generations = rawscope_data::DatasetGenerationCounter::default();
    let schema = DatasetSchema::try_new([("name", StoreColumnKind::Utf8)]).unwrap();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(1, 1),
        schema,
        DatasetMemoryBudget::new(10_000),
        &mut generations,
    );
    builder
        .append_chunk(DatasetChunk::new(
            RowId(0),
            vec![ColumnChunk::new(
                ColumnId::new(0),
                vec![StoredCell::value(
                    SourceValue::Utf8(Arc::from("alice")),
                    NormalizedValue::Text(Arc::from("alice")),
                )],
            )],
        ))
        .unwrap();
    let store = builder.finish();
    let key = DatasetEvidenceKey::validate(&store, ColumnId::new(0)).unwrap();
    assert_eq!(key.column_id(), ColumnId::new(0));
    assert!(matches!(
        key.value(&store, RowId(0)).unwrap(),
        CellRef::Value(SourceValue::Utf8(value)) if value.as_ref() == "alice"
    ));
}
