use proptest::prelude::*;
use rawscope_core::{ColumnId, RowId};
use rawscope_data::{ColumnChunk, DatasetChunk, DatasetSchema, StoreColumnKind, StoredCell};

proptest! {
    #[test]
    fn generated_chunks_are_rectangular_when_each_column_has_the_same_row_count(
        row_count in 1_usize..64,
        column_count in 1_usize..8,
    ) {
        let schema = DatasetSchema::try_new((0..column_count).map(|index| {
            (format!("column_{index}"), StoreColumnKind::Utf8)
        })).expect("generated column names are unique");
        let columns = (0..column_count).map(|column| {
            ColumnChunk::new(
                ColumnId::new(column as u32),
                (0..row_count)
                    .map(|row| StoredCell::value(
                        rawscope_data::SourceValue::Utf8(format!("{column}-{row}").into()),
                        rawscope_data::NormalizedValue::Text(format!("{column}-{row}").into()),
                    ))
                    .collect(),
            )
        }).collect::<Vec<_>>();
        let chunk = DatasetChunk::new(RowId(0), columns);

        prop_assert_eq!(chunk.row_count(), row_count);
        prop_assert!(chunk.validate_against(&schema, RowId(0)).is_ok());
    }
}
