use rawscope_core::RowId;
use rawscope_data::{
    validate_evidence_key, DatasetEvidenceKey, EvidenceKeyValidationError, LoadedColumnKind,
    LoadedColumnSchema, LoadedSourceRow, LoadedSourceTable,
};

#[test]
fn evidence_key_accepts_unique_non_blank_values() {
    let source = source(&[(0, "g-001"), (1, "g-002")]);

    let key = validate_evidence_key(&source, "game_id").unwrap();

    assert_eq!(
        key,
        DatasetEvidenceKey {
            column_name: "game_id".to_string(),
            column_index: 0,
        }
    );
    assert_eq!(key.value(&source.rows[1]), Some("g-002"));
}

#[test]
fn evidence_key_rejects_blank_value_with_row_context() {
    let source = source(&[(0, "g-001"), (7, "  ")]);

    let error = validate_evidence_key(&source, "game_id").unwrap_err();

    assert_eq!(
        error,
        EvidenceKeyValidationError::MissingValue {
            column_name: "game_id".to_string(),
            row_id: RowId(7),
        }
    );
}

#[test]
fn evidence_key_rejects_duplicate_value_with_both_rows() {
    let source = source(&[(4, "g-001"), (9, " g-001 ")]);

    let error = validate_evidence_key(&source, "game_id").unwrap_err();

    assert_eq!(
        error,
        EvidenceKeyValidationError::DuplicateValue {
            column_name: "game_id".to_string(),
            value: "g-001".to_string(),
            first: RowId(4),
            duplicate: RowId(9),
        }
    );
}

#[test]
fn evidence_key_does_not_replace_dense_internal_row_id() {
    let source = source(&[(0, "g-001"), (1, "g-002")]);

    let key = validate_evidence_key(&source, "game_id").unwrap();

    assert_eq!(source.rows[0].row_id, RowId(0));
    assert_eq!(source.rows[1].row_id, RowId(1));
    assert_eq!(key.value(&source.rows[0]), Some("g-001"));
}

#[test]
fn evidence_key_missing_column_lists_available_columns() {
    let source = source(&[(0, "g-001")]);

    let error = validate_evidence_key(&source, "missing").unwrap_err();

    assert!(matches!(
        error,
        EvidenceKeyValidationError::MissingColumn {
            column_name,
            available_columns
        } if column_name == "missing" && available_columns == vec!["game_id".to_string()]
    ));
}

fn source(rows: &[(u64, &str)]) -> LoadedSourceTable {
    LoadedSourceTable {
        columns: vec![LoadedColumnSchema {
            name: "game_id".to_string(),
            kind: LoadedColumnKind::String,
        }],
        rows: rows
            .iter()
            .map(|(row_id, value)| LoadedSourceRow {
                row_id: RowId(*row_id),
                values: vec![(*value).to_string()],
            })
            .collect(),
    }
}
