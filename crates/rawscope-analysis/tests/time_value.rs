use rawscope_analysis::visual_field::{
    ProjectedVisualFieldGeneration, VisualAxisDomain, VisualAxisSelectionRange,
    VisualFieldBrushSelection, VisualFieldMapping, VisualFieldProjection,
};
use rawscope_core::{ColumnId, RowId};
use rawscope_data::{
    CellState, ColumnChunk, DatasetChunk, DatasetGenerationCounter, DatasetIdentity,
    DatasetMemoryBudget, DatasetSchema, DatasetStore, DatasetStoreBuilder, NormalizedValue,
    SourceValue, StoreColumnKind, StoredCell,
};

fn time_value_store(times: &[i64], values: &[f64]) -> DatasetStore {
    assert_eq!(times.len(), values.len());
    let schema = DatasetSchema::try_new([
        ("observed_at", StoreColumnKind::TimestampMicros),
        ("value", StoreColumnKind::F64),
    ])
    .expect("time-value schema is valid");
    let time_cells = times
        .iter()
        .copied()
        .map(|value| {
            StoredCell::value(
                SourceValue::TimestampMicros(value),
                NormalizedValue::TimestampMicros(value),
            )
        })
        .collect();
    let value_cells = values
        .iter()
        .copied()
        .map(|value| StoredCell::value(SourceValue::F64(value), NormalizedValue::F64(value)))
        .collect();
    let columns = vec![
        ColumnChunk::new(ColumnId::new(0), time_cells),
        ColumnChunk::new(ColumnId::new(1), value_cells),
    ];
    let mut generations = DatasetGenerationCounter::default();
    let mut builder = DatasetStoreBuilder::new(
        DatasetIdentity::synthetic_scatter(11, times.len()),
        schema,
        DatasetMemoryBudget::new(1_000_000),
        &mut generations,
    );
    builder
        .append_chunk(DatasetChunk::new(RowId(0), columns))
        .expect("time-value fixture chunk is valid");
    builder.finish()
}

fn time_value_projection(store: &DatasetStore) -> VisualFieldMapping {
    VisualFieldMapping::try_new(
        store.schema(),
        VisualFieldProjection::TimeValue {
            time: ColumnId::new(0),
            value: ColumnId::new(1),
        },
        None,
    )
    .expect("time-value mapping is valid")
}

#[test]
fn time_value_projection_preserves_row_ids_and_typed_domains() {
    let store = time_value_store(&[-20, 30], &[1.5, 4.5]);
    let generation =
        ProjectedVisualFieldGeneration::from_store(&store, time_value_projection(&store))
            .expect("time-value projection should build");

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
        VisualAxisDomain::TimestampMicros { min: -20, max: 30 }
    );
    assert_eq!(generation.timestamp_micros().unwrap().as_ref(), &[-20, 30]);
}

#[test]
fn large_epoch_values_normalize_without_f32_epoch_loss() {
    let origin = 9_007_199_254_740_992_i64;
    let store = time_value_store(&[origin, origin + 1], &[2.0, 3.0]);
    let generation =
        ProjectedVisualFieldGeneration::from_store(&store, time_value_projection(&store))
            .expect("large-epoch time-value projection should build");

    let gpu_points = generation.gpu_points();
    assert_eq!(gpu_points[0].x, 0.0);
    assert_eq!(gpu_points[1].x, 1.0);
    assert_eq!(
        generation.timestamp_micros().unwrap().as_ref(),
        &[origin, origin + 1]
    );
}

#[test]
fn zero_span_time_domain_maps_to_a_finite_center() {
    let store = time_value_store(&[42, 42], &[1.0, 2.0]);
    let generation =
        ProjectedVisualFieldGeneration::from_store(&store, time_value_projection(&store))
            .expect("singleton time domain should remain valid");

    assert_eq!(
        generation.x_domain(),
        VisualAxisDomain::TimestampMicros { min: 42, max: 42 }
    );
    assert!(generation.gpu_points().iter().all(|point| point.x == 0.5));
    assert!(generation
        .time_axis_transform()
        .expect("time-value transform")
        .normalized(42)
        .is_finite());
}

#[test]
fn profile_identity_does_not_change_time_value_availability() {
    let store = time_value_store(&[1, 2], &[10.0, 20.0]);
    let mapping = time_value_projection(&store);
    let first = ProjectedVisualFieldGeneration::from_store(&store, mapping)
        .expect("time-value mapping does not require a profile");
    let second = ProjectedVisualFieldGeneration::from_store(&store, mapping)
        .expect("the same mapping remains profile-independent");

    assert_eq!(first.mapping(), second.mapping());
    assert_eq!(first.points(), second.points());
    assert!(matches!(
        store.cell(RowId(0), ColumnId::new(0)).unwrap().normalized(),
        CellState::Value(NormalizedValue::TimestampMicros(1))
    ));
}

#[test]
fn time_value_brush_uses_exact_timestamp_bounds() {
    let store = time_value_store(&[9_007_199_254_740_992, 9_007_199_254_740_993], &[1.0, 2.0]);
    let generation =
        ProjectedVisualFieldGeneration::from_store(&store, time_value_projection(&store))
            .expect("time-value projection should build");
    let selection = VisualFieldBrushSelection::new(
        VisualAxisSelectionRange::TimestampMicros {
            min: 9_007_199_254_740_993,
            max: 9_007_199_254_740_993,
        },
        VisualAxisSelectionRange::F64(
            rawscope_analysis::inspection::F64Domain::try_new(1.5, 2.5).unwrap(),
        ),
    );

    assert_eq!(
        generation.row_ids_for_brush(selection).unwrap().as_ref(),
        &[RowId(1)]
    );
}
