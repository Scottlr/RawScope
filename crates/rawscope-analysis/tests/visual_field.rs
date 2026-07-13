use std::sync::Arc;

use rawscope_analysis::visual_field::{
    ProjectedVisualFieldGeneration, TimeAxisTransform, VisualAxisDomain, VisualFieldMapping,
    VisualFieldMappingError, VisualFieldMode, VisualFieldModeSupport, VisualFieldProjection,
    VisualFieldRowPolicy,
};
use rawscope_core::{ColumnId, RowId};
use rawscope_data::{
    CellState, ColumnChunk, DatasetChunk, DatasetFieldBinding, DatasetFieldRole,
    DatasetGenerationCounter, DatasetIdentity, DatasetMemoryBudget, DatasetSchema, DatasetStore,
    DatasetStoreBuilder, InvalidCell, InvalidCellReason, NormalizedValue, SourceValue,
    StoreColumnKind, StoredCell,
};

fn schema(columns: &[(&str, StoreColumnKind)]) -> DatasetSchema {
    DatasetSchema::try_new(columns.iter().copied()).expect("fixture schema is valid")
}

fn value_cell(raw: SourceValue, normalized: NormalizedValue) -> StoredCell {
    StoredCell::value(raw, normalized)
}

fn store(schema: DatasetSchema, rows: Vec<Vec<StoredCell>>) -> DatasetStore {
    let row_count = rows.len();
    let column_count = schema.len();
    let columns = (0..column_count)
        .map(|column_index| {
            let cells = rows
                .iter()
                .map(|row| row[column_index].clone())
                .collect::<Vec<_>>();
            ColumnChunk::new(ColumnId::new(column_index as u32), cells)
        })
        .collect();
    let mut generations = DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(11, row_count),
        schema,
        DatasetMemoryBudget::new(1_000_000),
        &mut generations,
    );
    builder
        .append_chunk(DatasetChunk::new(RowId(0), columns))
        .expect("fixture chunk is valid");
    builder.finish()
}

#[test]
fn visual_field_mapping_accepts_profile_free_numeric_pair() {
    let numeric_schema = schema(&[
        ("horizontal", StoreColumnKind::I64),
        ("vertical", StoreColumnKind::F64),
        ("segment", StoreColumnKind::Utf8),
    ]);
    let mapping = VisualFieldMapping::try_new(
        &numeric_schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        Some(ColumnId::new(2)),
    )
    .expect("profile-free numeric mapping should validate");

    assert_eq!(mapping.category(), Some(ColumnId::new(2)));
    assert_eq!(
        mapping.dataset_field_bindings(&numeric_schema).unwrap(),
        vec![
            DatasetFieldBinding::new(DatasetFieldRole::X, "horizontal"),
            DatasetFieldBinding::new(DatasetFieldRole::Y, "vertical"),
            DatasetFieldBinding::new(DatasetFieldRole::Category, "segment"),
        ]
    );
    assert_eq!(
        mapping.supports(VisualFieldMode::CategoryComposition),
        VisualFieldModeSupport::Supported
    );
}

#[test]
fn visual_field_mapping_rejects_wrong_missing_and_duplicate_bindings() {
    let schema = schema(&[
        ("horizontal", StoreColumnKind::I64),
        ("vertical", StoreColumnKind::Utf8),
    ]);
    let wrong_kind = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        None,
    )
    .expect_err("text cannot be a numeric axis");
    assert!(matches!(
        wrong_kind,
        VisualFieldMappingError::AxisKindMismatch { .. }
    ));

    let missing = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(4),
        },
        None,
    )
    .expect_err("unknown columns must be rejected");
    assert!(matches!(
        missing,
        VisualFieldMappingError::MissingColumn { .. }
    ));

    let duplicate = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(0),
        },
        None,
    )
    .expect_err("one column cannot supply both axes");
    assert!(matches!(
        duplicate,
        VisualFieldMappingError::DuplicateAxes { .. }
    ));
}

#[test]
fn category_binding_accepts_integer_groups_and_cannot_alias_an_axis() {
    let numeric_schema = schema(&[
        ("horizontal", StoreColumnKind::I64),
        ("vertical", StoreColumnKind::F64),
        ("segment", StoreColumnKind::U64),
    ]);
    let mapping = VisualFieldMapping::try_new(
        &numeric_schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        Some(ColumnId::new(2)),
    )
    .expect("integer-coded category is exact");
    assert_eq!(mapping.category(), Some(ColumnId::new(2)));

    let aliased = VisualFieldMapping::try_new(
        &numeric_schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        Some(ColumnId::new(0)),
    )
    .expect_err("category cannot alias an axis");
    assert!(matches!(
        aliased,
        VisualFieldMappingError::CategoryMatchesAxis { .. }
    ));

    let wrong_category_schema = schema(&[
        ("horizontal", StoreColumnKind::I64),
        ("vertical", StoreColumnKind::F64),
        ("not-a-category", StoreColumnKind::F64),
    ]);
    let wrong_category = VisualFieldMapping::try_new(
        &wrong_category_schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        Some(ColumnId::new(2)),
    )
    .expect_err("continuous floats do not have category equality semantics");
    assert!(matches!(
        wrong_category,
        VisualFieldMappingError::CategoryKindMismatch { .. }
    ));

    let without_category = VisualFieldMapping::try_new(
        &numeric_schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        None,
    )
    .unwrap();
    assert_eq!(
        without_category.supports(VisualFieldMode::CategoryComposition),
        VisualFieldModeSupport::CategoryRequired
    );
    assert_eq!(
        without_category.require_mode(VisualFieldMode::CategoryComposition),
        Err(VisualFieldMappingError::CategoryRequired)
    );
}

#[test]
fn projected_generation_preserves_row_ids_and_typed_domains() {
    let schema = schema(&[
        ("horizontal", StoreColumnKind::I64),
        ("vertical", StoreColumnKind::U64),
    ]);
    let store = store(
        schema.clone(),
        vec![
            vec![
                value_cell(SourceValue::I64(-2), NormalizedValue::I64(-2)),
                value_cell(SourceValue::U64(10), NormalizedValue::U64(10)),
            ],
            vec![
                value_cell(SourceValue::I64(3), NormalizedValue::I64(3)),
                value_cell(SourceValue::U64(20), NormalizedValue::U64(20)),
            ],
        ],
    );
    let mapping = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        None,
    )
    .unwrap();
    let generation = ProjectedVisualFieldGeneration::from_store(&store, mapping).unwrap();

    assert_eq!(generation.dataset_generation(), store.generation());
    assert_eq!(
        generation
            .points()
            .iter()
            .map(|point| point.row_id)
            .collect::<Vec<_>>(),
        vec![RowId(0), RowId(1)]
    );
    assert_eq!(
        generation.x_domain(),
        VisualAxisDomain::I64 { min: -2, max: 3 }
    );
    assert_eq!(
        generation.y_domain(),
        VisualAxisDomain::U64 { min: 10, max: 20 }
    );
}

#[test]
fn time_value_mapping_retains_exact_timestamp_endpoints() {
    let schema = schema(&[
        ("observed_at", StoreColumnKind::TimestampMicros),
        ("value", StoreColumnKind::F64),
    ]);
    let store = store(
        schema.clone(),
        vec![
            vec![
                value_cell(
                    SourceValue::TimestampMicros(-9_007_199_254_740_992),
                    NormalizedValue::TimestampMicros(-9_007_199_254_740_992),
                ),
                value_cell(SourceValue::F64(1.5), NormalizedValue::F64(1.5)),
            ],
            vec![
                value_cell(
                    SourceValue::TimestampMicros(9_007_199_254_740_991),
                    NormalizedValue::TimestampMicros(9_007_199_254_740_991),
                ),
                value_cell(SourceValue::F64(3.5), NormalizedValue::F64(3.5)),
            ],
        ],
    );
    let mapping = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::TimeValue {
            time: ColumnId::new(0),
            value: ColumnId::new(1),
        },
        None,
    )
    .unwrap();
    let generation = ProjectedVisualFieldGeneration::from_store(&store, mapping).unwrap();

    assert_eq!(
        generation.x_domain(),
        VisualAxisDomain::TimestampMicros {
            min: -9_007_199_254_740_992,
            max: 9_007_199_254_740_991,
        }
    );
    assert_eq!(generation.points()[0].x, -9_007_199_254_740_992.0);
}

#[test]
fn time_value_brush_uses_exact_timestamp_bounds() {
    let transform = TimeAxisTransform::from_domain(10, 20).unwrap();
    assert_eq!(transform.denormalized_range(0.21, 0.29).unwrap(), (12, 13));
}

#[test]
fn invalid_and_missing_axis_cells_are_excluded_without_row_reallocation() {
    let schema = schema(&[
        ("horizontal", StoreColumnKind::I64),
        ("vertical", StoreColumnKind::F64),
    ]);
    let invalid = InvalidCell::new(RowId(1), ColumnId::new(0), InvalidCellReason::ParseFailure);
    let store = store(
        schema.clone(),
        vec![
            vec![
                value_cell(SourceValue::I64(1), NormalizedValue::I64(1)),
                value_cell(SourceValue::F64(2.0), NormalizedValue::F64(2.0)),
            ],
            vec![
                StoredCell::invalid(Some(SourceValue::Utf8(Arc::from("bad"))), invalid),
                value_cell(SourceValue::F64(3.0), NormalizedValue::F64(3.0)),
            ],
            vec![
                value_cell(SourceValue::I64(4), NormalizedValue::I64(4)),
                StoredCell::missing(None),
            ],
        ],
    );
    let mapping = VisualFieldMapping::try_new(
        &schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        None,
    )
    .unwrap();

    let generation = ProjectedVisualFieldGeneration::from_store(&store, mapping).unwrap();
    assert_eq!(generation.points().len(), 1);
    assert_eq!(generation.points()[0].row_id, RowId(0));
    assert!(matches!(
        ProjectedVisualFieldGeneration::from_store_with_policy(
            &store,
            mapping,
            VisualFieldRowPolicy::RejectMissingOrInvalid,
        ),
        Err(
            rawscope_analysis::visual_field::VisualFieldProjectionError::InvalidValue {
                row_id: RowId(1),
                ..
            }
        )
    ));
    assert!(matches!(
        store.cell(RowId(1), ColumnId::new(0)).unwrap().normalized(),
        CellState::Invalid(_)
    ));
}
