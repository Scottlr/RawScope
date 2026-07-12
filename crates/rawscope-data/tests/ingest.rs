use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use rawscope_core::RowId;
use rawscope_data::{
    CellRef, ChunkRowCount, DatasetGenerationCounter, DatasetIdentity, DatasetMemoryBudget,
    IngestionError, IngestionPlan, IngestionRequest, LoadedColumnKind, LoadedColumnSchema,
    LoadedSourceRow, LoadedSourceTable,
};

fn request() -> IngestionRequest {
    IngestionRequest {
        identity: DatasetIdentity::synthetic_scatter(7, 3),
        source: LoadedSourceTable {
            columns: vec![
                LoadedColumnSchema {
                    name: "rating".to_string(),
                    kind: LoadedColumnKind::Integer,
                },
                LoadedColumnSchema {
                    name: "label".to_string(),
                    kind: LoadedColumnKind::String,
                },
            ],
            rows: vec![
                LoadedSourceRow {
                    row_id: RowId(0),
                    values: vec![" 42 ".to_string(), " first ".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(1),
                    values: vec!["".to_string(), "second".to_string()],
                },
                LoadedSourceRow {
                    row_id: RowId(2),
                    values: vec!["99".to_string(), "third".to_string()],
                },
            ],
        },
        chunk_rows: ChunkRowCount::new(2).unwrap(),
        memory_budget: DatasetMemoryBudget::new(10_000),
    }
}

#[test]
fn staged_ingestion_preserves_rows_and_reports_chunk_progress() {
    let plan = IngestionPlan::prepare(request()).unwrap();
    let mut generations = DatasetGenerationCounter::default();
    let mut progress = Vec::new();
    let store = plan
        .execute(
            &|| false,
            |value| progress.push((value.rows_processed, value.total_rows)),
            &mut generations,
        )
        .unwrap();
    assert_eq!(progress, vec![(2, 3), (3, 3)]);
    assert_eq!(store.row_count(), 3);
    assert!(matches!(
        store.source_value(RowId(0), rawscope_core::ColumnId::new(0)).unwrap(),
        CellRef::Value(rawscope_data::SourceValue::Utf8(value)) if value.as_ref() == " 42 "
    ));
    assert!(matches!(
        store
            .source_value(RowId(1), rawscope_core::ColumnId::new(0))
            .unwrap(),
        CellRef::Missing
    ));
}

#[test]
fn cancellation_discards_the_partial_store_before_public_commit() {
    let plan = IngestionPlan::prepare(request()).unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let callback_flag = Arc::clone(&cancelled);
    let mut generations = DatasetGenerationCounter::default();
    let error = plan
        .execute(
            &|| cancelled.load(Ordering::Acquire),
            |progress| {
                if progress.rows_processed == 2 {
                    callback_flag.store(true, Ordering::Release);
                }
            },
            &mut generations,
        )
        .unwrap_err();
    assert_eq!(error, IngestionError::Cancelled { rows_processed: 2 });
}
