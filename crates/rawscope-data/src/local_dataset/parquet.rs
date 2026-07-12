//! Parquet loading and column binding for local workbench datasets.

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use arrow_array::{
    Array, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
    LargeStringArray, RecordBatch, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use arrow_schema::{DataType, Schema};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rawscope_core::{F32Range, RowId, U64Range};

use crate::{
    local_dataset::csv::{csv_row_number, lane_id_for_value},
    local_dataset::{
        DatasetChunkId, DatasetLoadError, LoadedColumnKind, LoadedColumnSchema,
        LoadedColumnarChunk, LoadedColumnarDataset, LoadedScatterDataset, LoadedSourceRow,
        LoadedSourceTable, LoadedTimelineDataset, PARQUET_RECORD_BATCH_SIZE_ROWS,
        SCATTER_NUMERIC_TYPE_EXPECTATION, TIMELINE_LANE_TYPE_EXPECTATION,
        TIMELINE_TIME_TYPE_EXPECTATION,
    },
    DatasetIdentity, ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};

/// Loads a local scatter dataset from Parquet and binds explicit x/y columns.
pub fn load_parquet_scatter_dataset(
    path: impl AsRef<Path>,
    x_column: &str,
    y_column: &str,
    limit: Option<usize>,
) -> Result<LoadedScatterDataset, DatasetLoadError> {
    let path = path.as_ref();
    let parquet = read_parquet_batches(path, limit)?;
    let x_index = column_index(&parquet.arrow_schema, x_column)?;
    let y_index = column_index(&parquet.arrow_schema, y_column)?;
    ensure_numeric_column(&parquet.arrow_schema, x_index, x_column)?;
    ensure_numeric_column(&parquet.arrow_schema, y_index, y_column)?;

    let mut points = Vec::new();
    let mut source_rows = Vec::new();
    let mut chunks = Vec::new();
    let mut global_row_offset = 0usize;
    let mut x_min = f32::INFINITY;
    let mut x_max = f32::NEG_INFINITY;
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;

    for (chunk_index, batch) in parquet.batches.iter().enumerate() {
        let chunk_row_count = batch.num_rows();
        if chunk_row_count == 0 {
            continue;
        }

        chunks.push(LoadedColumnarChunk {
            chunk_id: DatasetChunkId(chunk_index as u32),
            row_id_start: RowId(global_row_offset as u64),
            row_count: chunk_row_count,
        });

        let x_array = batch.column(x_index).as_ref();
        let y_array = batch.column(y_index).as_ref();

        for row_index in 0..chunk_row_count {
            let row_number = parquet_row_number(global_row_offset, row_index);
            let x =
                numeric_cell_as_f32(path, chunk_index, x_array, row_index, x_column, row_number)?;
            let y =
                numeric_cell_as_f32(path, chunk_index, y_array, row_index, y_column, row_number)?;
            let row_id = RowId((global_row_offset + row_index) as u64);

            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);
            points.push(ScatterPointRecord {
                row_id,
                x,
                y,
                kind: ScatterPointKind::Unclassified,
            });
            source_rows.push(LoadedSourceRow {
                row_id,
                values: source_row_values(path, chunk_index, batch, row_index)?,
            });
        }

        global_row_offset += chunk_row_count;
    }

    if points.is_empty() {
        return Err(DatasetLoadError::EmptyDataset {
            path: path.to_path_buf(),
        });
    }

    let identity = DatasetIdentity::local_parquet_scatter(
        path.to_path_buf(),
        points.len(),
        limit,
        x_column,
        y_column,
    );
    let schema = parquet_schema(&parquet.arrow_schema);
    let columnar = LoadedColumnarDataset {
        identity: identity.clone(),
        schema: schema.clone(),
        chunks,
    };

    Ok(LoadedScatterDataset {
        identity,
        schema: schema.clone(),
        columnar: Some(columnar),
        source_rows: LoadedSourceTable {
            columns: schema,
            rows: source_rows,
        },
        x_column: x_column.to_string(),
        y_column: y_column.to_string(),
        x_range: F32Range::from_bounds_expanded(x_min, x_max),
        y_range: F32Range::from_bounds_expanded(y_min, y_max),
        points,
    })
}

/// Loads only the inferred Parquet schema needed for profile validation.
pub fn load_dataset_schema(
    path: impl AsRef<Path>,
    _limit: Option<usize>,
) -> Result<Vec<LoadedColumnSchema>, DatasetLoadError> {
    let path = path.as_ref();
    let schema = read_parquet_schema(path)?;
    reject_float16_schema(&schema)?;
    Ok(parquet_schema(&schema))
}

/// Loads a local timeline dataset from Parquet and binds explicit time/lane columns.
pub fn load_parquet_timeline_dataset(
    path: impl AsRef<Path>,
    time_column: &str,
    lane_column: &str,
    limit: Option<usize>,
) -> Result<LoadedTimelineDataset, DatasetLoadError> {
    let path = path.as_ref();
    let parquet = read_parquet_batches(path, limit)?;
    let time_index = column_index(&parquet.arrow_schema, time_column)?;
    let lane_index = column_index(&parquet.arrow_schema, lane_column)?;
    ensure_integer_time_column(&parquet.arrow_schema, time_index, time_column)?;
    ensure_lane_column(&parquet.arrow_schema, lane_index, lane_column)?;

    let mut events = Vec::new();
    let mut source_rows = Vec::new();
    let mut chunks = Vec::new();
    let mut global_row_offset = 0usize;
    let mut lane_ids = HashMap::new();
    let mut lane_labels = Vec::new();
    let mut time_min = u64::MAX;
    let mut time_max = 0u64;

    for (chunk_index, batch) in parquet.batches.iter().enumerate() {
        let chunk_row_count = batch.num_rows();
        if chunk_row_count == 0 {
            continue;
        }

        chunks.push(LoadedColumnarChunk {
            chunk_id: DatasetChunkId(chunk_index as u32),
            row_id_start: RowId(global_row_offset as u64),
            row_count: chunk_row_count,
        });

        let time_array = batch.column(time_index).as_ref();
        let lane_array = batch.column(lane_index).as_ref();

        for row_index in 0..chunk_row_count {
            let row_number = parquet_row_number(global_row_offset, row_index);
            let timestamp = integer_timestamp_cell(
                path,
                chunk_index,
                time_array,
                row_index,
                time_column,
                row_number,
            )?;
            let lane_value = lane_cell_as_string(
                path,
                chunk_index,
                lane_array,
                row_index,
                lane_column,
                row_number,
            )?;
            let lane = lane_id_for_value(lane_value, &mut lane_ids, &mut lane_labels)?;
            let row_id = RowId((global_row_offset + row_index) as u64);

            time_min = time_min.min(timestamp);
            time_max = time_max.max(timestamp);
            events.push(TimelineEventRecord {
                row_id,
                timestamp,
                lane,
                value: 1.0,
                kind: TimelineEventKind::Unclassified,
            });
            source_rows.push(LoadedSourceRow {
                row_id,
                values: source_row_values(path, chunk_index, batch, row_index)?,
            });
        }

        global_row_offset += chunk_row_count;
    }

    if events.is_empty() {
        return Err(DatasetLoadError::EmptyDataset {
            path: path.to_path_buf(),
        });
    }

    let identity = DatasetIdentity::local_parquet_timeline(
        path.to_path_buf(),
        events.len(),
        limit,
        time_column,
        lane_column,
        lane_labels.clone(),
    );
    let schema = parquet_schema(&parquet.arrow_schema);
    let columnar = LoadedColumnarDataset {
        identity: identity.clone(),
        schema: schema.clone(),
        chunks,
    };

    Ok(LoadedTimelineDataset {
        identity,
        schema: schema.clone(),
        columnar: Some(columnar),
        source_rows: LoadedSourceTable {
            columns: schema,
            rows: source_rows,
        },
        time_column: time_column.to_string(),
        lane_column: lane_column.to_string(),
        time_range: U64Range::from_bounds_expanded(time_min, time_max),
        lane_count: lane_labels.len() as u32,
        lane_labels,
        events,
    })
}

struct LoadedParquetBatches {
    arrow_schema: Schema,
    batches: Vec<RecordBatch>,
}

fn read_parquet_schema(path: &Path) -> Result<Schema, DatasetLoadError> {
    let file = File::open(path).map_err(|source| parquet_read_error(path, source.to_string()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|source| {
        DatasetLoadError::ParquetRead {
            path: path.to_path_buf(),
            source,
        }
    })?;
    Ok(builder.schema().as_ref().clone())
}

fn read_parquet_batches(
    path: &Path,
    limit: Option<usize>,
) -> Result<LoadedParquetBatches, DatasetLoadError> {
    let file = File::open(path).map_err(|source| parquet_read_error(path, source.to_string()))?;
    let mut builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|source| {
        DatasetLoadError::ParquetRead {
            path: path.to_path_buf(),
            source,
        }
    })?;
    builder = builder.with_batch_size(PARQUET_RECORD_BATCH_SIZE_ROWS);
    if let Some(max_rows) = limit {
        builder = builder.with_limit(max_rows);
    }
    let arrow_schema = builder.schema().as_ref().clone();
    reject_float16_schema(&arrow_schema)?;
    let reader = builder
        .build()
        .map_err(|source| DatasetLoadError::ParquetRead {
            path: path.to_path_buf(),
            source,
        })?;
    let mut batches = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|source| DatasetLoadError::ParquetRead {
            path: path.to_path_buf(),
            source: source.into(),
        })?;
        if batch.num_rows() > 0 {
            batches.push(batch);
        }
    }

    Ok(LoadedParquetBatches {
        arrow_schema,
        batches,
    })
}

fn parquet_schema(arrow_schema: &Schema) -> Vec<LoadedColumnSchema> {
    arrow_schema
        .fields()
        .iter()
        .map(|field| LoadedColumnSchema {
            name: field.name().to_string(),
            kind: loaded_column_kind(field.data_type()),
        })
        .collect()
}

fn loaded_column_kind(data_type: &DataType) -> LoadedColumnKind {
    match data_type {
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => LoadedColumnKind::Integer,
        DataType::Float32 | DataType::Float64 => LoadedColumnKind::Float,
        DataType::Null => LoadedColumnKind::Empty,
        DataType::Utf8 | DataType::LargeUtf8 => LoadedColumnKind::String,
        _ => LoadedColumnKind::Unsupported,
    }
}

fn column_index(schema: &Schema, column: &str) -> Result<usize, DatasetLoadError> {
    schema
        .fields()
        .iter()
        .position(|field| field.name() == column)
        .ok_or_else(|| DatasetLoadError::MissingColumn {
            column: column.to_string(),
            available_columns: schema
                .fields()
                .iter()
                .map(|field| field.name().to_string())
                .collect(),
        })
}

fn ensure_numeric_column(
    schema: &Schema,
    column_index: usize,
    column: &str,
) -> Result<(), DatasetLoadError> {
    let data_type = schema.field(column_index).data_type();
    if matches!(
        data_type,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64
    ) {
        return Ok(());
    }

    Err(DatasetLoadError::UnsupportedParquetColumnType {
        column: column.to_string(),
        expected: SCATTER_NUMERIC_TYPE_EXPECTATION,
        actual: data_type.to_string(),
    })
}

fn ensure_integer_time_column(
    schema: &Schema,
    column_index: usize,
    column: &str,
) -> Result<(), DatasetLoadError> {
    let data_type = schema.field(column_index).data_type();
    if matches!(
        data_type,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
    ) {
        return Ok(());
    }

    Err(DatasetLoadError::UnsupportedParquetColumnType {
        column: column.to_string(),
        expected: TIMELINE_TIME_TYPE_EXPECTATION,
        actual: data_type.to_string(),
    })
}

fn ensure_lane_column(
    schema: &Schema,
    column_index: usize,
    column: &str,
) -> Result<(), DatasetLoadError> {
    let data_type = schema.field(column_index).data_type();
    if matches!(
        data_type,
        DataType::Utf8
            | DataType::LargeUtf8
            | DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
    ) {
        return Ok(());
    }

    Err(DatasetLoadError::UnsupportedParquetColumnType {
        column: column.to_string(),
        expected: TIMELINE_LANE_TYPE_EXPECTATION,
        actual: data_type.to_string(),
    })
}

fn numeric_cell_as_f32(
    path: &Path,
    batch_index: usize,
    array: &dyn Array,
    row_index: usize,
    column: &str,
    row_number: usize,
) -> Result<f32, DatasetLoadError> {
    if array.is_null(row_index) {
        return Err(parquet_invalid_value(
            path,
            batch_index,
            row_number,
            column,
            "",
            "a non-empty value",
        ));
    }

    let parsed = match array.data_type() {
        DataType::Int8 => array
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::Int16 => array
            .as_any()
            .downcast_ref::<Int16Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::Int32 => array
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::Int64 => array
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::UInt8 => array
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::UInt16 => array
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::UInt32 => array
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::UInt64 => array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(row_index) as f32,
        DataType::Float32 => array
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(row_index),
        DataType::Float64 => array
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .value(row_index) as f32,
        _ => {
            return Err(DatasetLoadError::UnsupportedParquetColumnType {
                column: column.to_string(),
                expected: SCATTER_NUMERIC_TYPE_EXPECTATION,
                actual: array.data_type().to_string(),
            })
        }
    };

    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(parquet_invalid_value(
            path,
            batch_index,
            row_number,
            column,
            &display_array_value(array, row_index)?,
            SCATTER_NUMERIC_TYPE_EXPECTATION,
        ))
    }
}

fn integer_timestamp_cell(
    path: &Path,
    batch_index: usize,
    array: &dyn Array,
    row_index: usize,
    column: &str,
    row_number: usize,
) -> Result<u64, DatasetLoadError> {
    if array.is_null(row_index) {
        return Err(parquet_invalid_value(
            path,
            batch_index,
            row_number,
            column,
            "",
            "a non-empty value",
        ));
    }

    match array.data_type() {
        DataType::Int8 => signed_timestamp(
            array
                .as_any()
                .downcast_ref::<Int8Array>()
                .unwrap()
                .value(row_index) as i64,
            path,
            batch_index,
            column,
            row_number,
        ),
        DataType::Int16 => signed_timestamp(
            array
                .as_any()
                .downcast_ref::<Int16Array>()
                .unwrap()
                .value(row_index) as i64,
            path,
            batch_index,
            column,
            row_number,
        ),
        DataType::Int32 => signed_timestamp(
            array
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap()
                .value(row_index) as i64,
            path,
            batch_index,
            column,
            row_number,
        ),
        DataType::Int64 => signed_timestamp(
            array
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap()
                .value(row_index),
            path,
            batch_index,
            column,
            row_number,
        ),
        DataType::UInt8 => Ok(array
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap()
            .value(row_index) as u64),
        DataType::UInt16 => Ok(array
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap()
            .value(row_index) as u64),
        DataType::UInt32 => Ok(array
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap()
            .value(row_index) as u64),
        DataType::UInt64 => Ok(array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(row_index)),
        _ => Err(DatasetLoadError::UnsupportedParquetColumnType {
            column: column.to_string(),
            expected: TIMELINE_TIME_TYPE_EXPECTATION,
            actual: array.data_type().to_string(),
        }),
    }
}

fn signed_timestamp(
    value: i64,
    path: &Path,
    batch_index: usize,
    column: &str,
    row_number: usize,
) -> Result<u64, DatasetLoadError> {
    u64::try_from(value).map_err(|_| {
        parquet_invalid_value(
            path,
            batch_index,
            row_number,
            column,
            &value.to_string(),
            TIMELINE_TIME_TYPE_EXPECTATION,
        )
    })
}

fn lane_cell_as_string(
    path: &Path,
    batch_index: usize,
    array: &dyn Array,
    row_index: usize,
    column: &str,
    row_number: usize,
) -> Result<String, DatasetLoadError> {
    if array.is_null(row_index) {
        return Err(parquet_invalid_value(
            path,
            batch_index,
            row_number,
            column,
            "",
            "a non-empty value",
        ));
    }

    match array.data_type() {
        DataType::Utf8 => Ok(array
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::LargeUtf8 => Ok(array
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int8 => Ok(array
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int16 => Ok(array
            .as_any()
            .downcast_ref::<Int16Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int32 => Ok(array
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int64 => Ok(array
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt8 => Ok(array
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt16 => Ok(array
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt32 => Ok(array
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt64 => Ok(array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        _ => Err(DatasetLoadError::UnsupportedParquetColumnType {
            column: column.to_string(),
            expected: TIMELINE_LANE_TYPE_EXPECTATION,
            actual: array.data_type().to_string(),
        }),
    }
}

fn source_row_values(
    path: &Path,
    batch_index: usize,
    batch: &RecordBatch,
    row_index: usize,
) -> Result<Vec<String>, DatasetLoadError> {
    batch
        .columns()
        .iter()
        .enumerate()
        .map(|(column_index, column)| {
            if !is_display_supported(column.data_type()) {
                return Err(DatasetLoadError::ParquetInvalidColumnValue {
                    path: path.to_path_buf(),
                    batch_index,
                    row_index,
                    column: batch.schema().field(column_index).name().to_string(),
                    value: column.data_type().to_string(),
                    expected: "a supported Parquet value type",
                });
            }
            display_array_value(column.as_ref(), row_index)
        })
        .collect()
}

fn reject_float16_schema(schema: &Schema) -> Result<(), DatasetLoadError> {
    if let Some(field) = schema
        .fields()
        .iter()
        .find(|field| matches!(field.data_type(), DataType::Float16))
    {
        return Err(DatasetLoadError::UnsupportedParquetColumnType {
            column: field.name().to_string(),
            expected: "supported Parquet scalar types (Float16 is not supported)",
            actual: field.data_type().to_string(),
        });
    }
    Ok(())
}

fn is_display_supported(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Utf8
            | DataType::LargeUtf8
            | DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64
            | DataType::Null
    )
}

fn display_array_value(array: &dyn Array, row_index: usize) -> Result<String, DatasetLoadError> {
    if array.is_null(row_index) {
        return Ok(String::new());
    }

    match array.data_type() {
        DataType::Utf8 => Ok(array
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::LargeUtf8 => Ok(array
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int8 => Ok(array
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int16 => Ok(array
            .as_any()
            .downcast_ref::<Int16Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int32 => Ok(array
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Int64 => Ok(array
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt8 => Ok(array
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt16 => Ok(array
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt32 => Ok(array
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::UInt64 => Ok(array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Float32 => Ok(array
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        DataType::Float64 => Ok(array
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .value(row_index)
            .to_string()),
        _ => Ok(format!("<{}>", array.data_type())),
    }
}

fn parquet_read_error(path: &Path, message: String) -> DatasetLoadError {
    DatasetLoadError::ParquetRead {
        path: PathBuf::from(path),
        source: parquet::errors::ParquetError::General(message),
    }
}

fn parquet_invalid_value(
    path: &Path,
    batch_index: usize,
    row_index: usize,
    column: &str,
    value: &str,
    expected: &'static str,
) -> DatasetLoadError {
    DatasetLoadError::ParquetInvalidColumnValue {
        path: path.to_path_buf(),
        batch_index,
        row_index,
        column: column.to_string(),
        value: value.to_string(),
        expected,
    }
}

fn parquet_row_number(global_row_offset: usize, chunk_row_index: usize) -> usize {
    csv_row_number(global_row_offset + chunk_row_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use arrow_array::{ArrayRef, BooleanArray, RecordBatch};
    use arrow_schema::{Field, Schema};

    #[test]
    fn float16_is_classified_as_unsupported() {
        assert_eq!(
            loaded_column_kind(&DataType::Float16),
            LoadedColumnKind::Unsupported
        );
        assert!(reject_float16_schema(&Schema::new(vec![Field::new(
            "value",
            DataType::Float16,
            false,
        )]))
        .is_err());
    }

    #[test]
    fn unsupported_source_values_return_parquet_context() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "flag",
            DataType::Boolean,
            false,
        )]));
        let batch = RecordBatch::try_new(
            schema,
            vec![Arc::new(BooleanArray::from(vec![true])) as ArrayRef],
        )
        .unwrap();

        let error = source_row_values(Path::new("fixture.parquet"), 2, &batch, 4)
            .expect_err("unsupported source values must not be fabricated");
        assert!(matches!(
            error,
            DatasetLoadError::ParquetInvalidColumnValue {
                batch_index: 2,
                row_index: 4,
                column,
                value,
                ..
            } if column == "flag" && value == "Boolean"
        ));
    }
}
