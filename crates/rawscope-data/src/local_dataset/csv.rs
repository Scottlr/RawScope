//! CSV loading and column binding for local workbench datasets.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use rawscope_core::{F32Range, RowId, U64Range};

use crate::{
    local_dataset::{
        ensure_supported_csv, DatasetLoadError, LoadedColumnKind, LoadedColumnSchema,
        LoadedScatterDataset, LoadedSourceRow, LoadedSourceTable, LoadedTimelineDataset,
        SCATTER_NUMERIC_TYPE_EXPECTATION, TIMELINE_LANE_TYPE_EXPECTATION,
        TIMELINE_TIME_TYPE_EXPECTATION,
    },
    DatasetIdentity, ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};

/// Loads a local scatter dataset from CSV and binds explicit x/y columns.
pub fn load_scatter_dataset(
    path: impl AsRef<Path>,
    x_column: &str,
    y_column: &str,
    limit: Option<usize>,
) -> Result<LoadedScatterDataset, DatasetLoadError> {
    let path = path.as_ref();
    ensure_supported_csv(path)?;
    let table = read_csv_table(path, limit)?;
    let x_index = table.column_index(x_column)?;
    let y_index = table.column_index(y_column)?;
    table.ensure_column_kind(
        x_index,
        SCATTER_NUMERIC_TYPE_EXPECTATION,
        LoadedColumnKind::is_numeric,
    )?;
    table.ensure_column_kind(
        y_index,
        SCATTER_NUMERIC_TYPE_EXPECTATION,
        LoadedColumnKind::is_numeric,
    )?;

    let mut points = Vec::with_capacity(table.rows.len());
    let source_rows = source_table(&table.schema, &table.rows);
    let mut x_min = f32::INFINITY;
    let mut x_max = f32::NEG_INFINITY;
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;

    for (row_offset, row) in table.rows.iter().enumerate() {
        let row_number = csv_row_number(row_offset);
        let x = parse_f32_cell(row, x_index, x_column, row_number)?;
        let y = parse_f32_cell(row, y_index, y_column, row_number)?;
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
        points.push(ScatterPointRecord {
            row_id: RowId(row_offset as u64),
            x,
            y,
            kind: ScatterPointKind::Unclassified,
        });
    }

    Ok(LoadedScatterDataset {
        identity: DatasetIdentity::local_csv_scatter(
            path.to_path_buf(),
            points.len(),
            limit,
            x_column,
            y_column,
        ),
        schema: table.schema,
        columnar: None,
        source_rows,
        x_column: x_column.to_string(),
        y_column: y_column.to_string(),
        x_range: F32Range::from_bounds_expanded(x_min, x_max),
        y_range: F32Range::from_bounds_expanded(y_min, y_max),
        points,
    })
}

/// Loads only the inferred CSV schema needed for profile validation.
pub fn load_dataset_schema(
    path: impl AsRef<Path>,
    limit: Option<usize>,
) -> Result<Vec<LoadedColumnSchema>, DatasetLoadError> {
    let path = path.as_ref();
    ensure_supported_csv(path)?;
    Ok(read_csv_table(path, limit)?.schema)
}

/// Loads a local timeline dataset from CSV and binds explicit time/lane columns.
pub fn load_timeline_dataset(
    path: impl AsRef<Path>,
    time_column: &str,
    lane_column: &str,
    limit: Option<usize>,
) -> Result<LoadedTimelineDataset, DatasetLoadError> {
    let path = path.as_ref();
    ensure_supported_csv(path)?;
    let table = read_csv_table(path, limit)?;
    let time_index = table.column_index(time_column)?;
    let lane_index = table.column_index(lane_column)?;
    table.ensure_column_kind(
        time_index,
        TIMELINE_TIME_TYPE_EXPECTATION,
        LoadedColumnKind::is_integer,
    )?;
    table.ensure_column_kind(
        lane_index,
        TIMELINE_LANE_TYPE_EXPECTATION,
        LoadedColumnKind::is_string_or_integer,
    )?;
    let lane_kind = table.schema[lane_index].kind;

    let mut events = Vec::with_capacity(table.rows.len());
    let source_rows = source_table(&table.schema, &table.rows);
    let mut lane_ids = HashMap::new();
    let mut lane_labels = Vec::new();
    let mut time_min = u64::MAX;
    let mut time_max = 0;

    for (row_offset, row) in table.rows.iter().enumerate() {
        let row_number = csv_row_number(row_offset);
        let timestamp = parse_u64_cell(row, time_index, time_column, row_number)?;
        let lane_value = parse_lane_cell(row, lane_index, lane_column, lane_kind, row_number)?;
        let lane = lane_id_for_value(lane_value, &mut lane_ids, &mut lane_labels)?;
        time_min = time_min.min(timestamp);
        time_max = time_max.max(timestamp);
        events.push(TimelineEventRecord {
            row_id: RowId(row_offset as u64),
            timestamp,
            lane,
            value: 1.0,
            kind: TimelineEventKind::Unclassified,
        });
    }

    Ok(LoadedTimelineDataset {
        identity: DatasetIdentity::local_csv_timeline(
            path.to_path_buf(),
            events.len(),
            limit,
            time_column,
            lane_column,
            lane_labels.clone(),
        ),
        schema: table.schema,
        columnar: None,
        source_rows,
        time_column: time_column.to_string(),
        lane_column: lane_column.to_string(),
        time_range: U64Range::from_bounds_expanded(time_min, time_max),
        lane_count: lane_labels.len() as u32,
        lane_labels,
        events,
    })
}

fn source_table(schema: &[LoadedColumnSchema], rows: &[::csv::StringRecord]) -> LoadedSourceTable {
    let columns = schema.to_vec();
    let rows = rows
        .iter()
        .enumerate()
        .map(|(row_offset, row)| LoadedSourceRow {
            row_id: RowId(row_offset as u64),
            values: (0..schema.len())
                .map(|column_index| row.get(column_index).unwrap_or("").trim().to_string())
                .collect(),
        })
        .collect();

    LoadedSourceTable { columns, rows }
}

struct CsvTable {
    headers: Vec<String>,
    schema: Vec<LoadedColumnSchema>,
    rows: Vec<::csv::StringRecord>,
}

impl CsvTable {
    fn column_index(&self, column: &str) -> Result<usize, DatasetLoadError> {
        self.headers
            .iter()
            .position(|header| header == column)
            .ok_or_else(|| DatasetLoadError::MissingColumn {
                column: column.to_string(),
                available_columns: self.headers.clone(),
            })
    }

    fn ensure_column_kind(
        &self,
        column_index: usize,
        expected: &'static str,
        accepts: impl FnOnce(LoadedColumnKind) -> bool,
    ) -> Result<(), DatasetLoadError> {
        let schema = &self.schema[column_index];
        if accepts(schema.kind) {
            return Ok(());
        }

        Err(DatasetLoadError::UnsupportedColumnType {
            column: schema.name.clone(),
            expected,
            actual: schema.kind,
        })
    }
}

fn read_csv_table(path: &Path, limit: Option<usize>) -> Result<CsvTable, DatasetLoadError> {
    let mut reader = ::csv::Reader::from_path(path).map_err(|source| csv_error(path, source))?;
    let headers = reader
        .headers()
        .map_err(|source| csv_error(path, source))?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();

    for record in reader.records() {
        let record = record.map_err(|source| csv_error(path, source))?;
        rows.push(record);
        let reached_limit = limit.is_some_and(|max_rows| rows.len() >= max_rows);
        if reached_limit {
            break;
        }
    }

    if rows.is_empty() {
        return Err(DatasetLoadError::EmptyDataset {
            path: path.to_path_buf(),
        });
    }

    let schema = infer_schema(&headers, &rows);
    Ok(CsvTable {
        headers,
        schema,
        rows,
    })
}

fn infer_schema(headers: &[String], rows: &[::csv::StringRecord]) -> Vec<LoadedColumnSchema> {
    headers
        .iter()
        .enumerate()
        .map(|(column_index, name)| {
            let kind = rows.iter().fold(LoadedColumnKind::Empty, |kind, row| {
                let value = row.get(column_index).unwrap_or("");
                infer_column_kind(kind, value)
            });
            LoadedColumnSchema {
                name: name.clone(),
                kind,
            }
        })
        .collect()
}

fn infer_column_kind(current: LoadedColumnKind, value: &str) -> LoadedColumnKind {
    let value = value.trim();
    if value.is_empty() || current == LoadedColumnKind::String {
        return current;
    }

    if value.parse::<i64>().is_ok() {
        return match current {
            LoadedColumnKind::Empty => LoadedColumnKind::Integer,
            LoadedColumnKind::Integer | LoadedColumnKind::Float => current,
            LoadedColumnKind::String => LoadedColumnKind::String,
        };
    }

    let is_finite_float = value
        .parse::<f64>()
        .map(|parsed| parsed.is_finite())
        .unwrap_or(false);
    if is_finite_float {
        return LoadedColumnKind::Float;
    }

    LoadedColumnKind::String
}

fn parse_f32_cell(
    row: &::csv::StringRecord,
    column_index: usize,
    column: &str,
    row_number: usize,
) -> Result<f32, DatasetLoadError> {
    let value = cell_value(row, column_index, column, row_number)?;
    let parsed = value
        .parse::<f32>()
        .map_err(|_| invalid_value(column, row_number, value, SCATTER_NUMERIC_TYPE_EXPECTATION))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(invalid_value(
            column,
            row_number,
            value,
            SCATTER_NUMERIC_TYPE_EXPECTATION,
        ))
    }
}

fn parse_u64_cell(
    row: &::csv::StringRecord,
    column_index: usize,
    column: &str,
    row_number: usize,
) -> Result<u64, DatasetLoadError> {
    let value = cell_value(row, column_index, column, row_number)?;
    value
        .parse::<u64>()
        .map_err(|_| invalid_value(column, row_number, value, TIMELINE_TIME_TYPE_EXPECTATION))
}

fn parse_lane_cell(
    row: &::csv::StringRecord,
    column_index: usize,
    column: &str,
    kind: LoadedColumnKind,
    row_number: usize,
) -> Result<String, DatasetLoadError> {
    let value = cell_value(row, column_index, column, row_number)?;
    if kind == LoadedColumnKind::Integer {
        let lane = value.parse::<u64>().map_err(|_| {
            invalid_value(column, row_number, value, TIMELINE_LANE_TYPE_EXPECTATION)
        })?;
        return Ok(lane.to_string());
    }

    Ok(value.to_string())
}

fn cell_value<'a>(
    row: &'a ::csv::StringRecord,
    column_index: usize,
    column: &str,
    row_number: usize,
) -> Result<&'a str, DatasetLoadError> {
    let value = row.get(column_index).unwrap_or("").trim();
    if value.is_empty() {
        return Err(invalid_value(
            column,
            row_number,
            value,
            "a non-empty value",
        ));
    }

    Ok(value)
}

pub(super) fn lane_id_for_value(
    value: String,
    lane_ids: &mut HashMap<String, u32>,
    lane_labels: &mut Vec<String>,
) -> Result<u32, DatasetLoadError> {
    if let Some(lane) = lane_ids.get(&value) {
        return Ok(*lane);
    }

    let next_lane =
        u32::try_from(lane_labels.len()).map_err(|_| DatasetLoadError::TooManyLanes {
            lane_count: lane_labels.len(),
        })?;
    lane_ids.insert(value.clone(), next_lane);
    lane_labels.push(value);
    Ok(next_lane)
}

pub(super) fn invalid_value(
    column: &str,
    row_number: usize,
    value: &str,
    expected: &'static str,
) -> DatasetLoadError {
    DatasetLoadError::InvalidColumnValue {
        column: column.to_string(),
        row_number,
        value: value.to_string(),
        expected,
    }
}

fn csv_error(path: &Path, source: ::csv::Error) -> DatasetLoadError {
    DatasetLoadError::CsvRead {
        path: PathBuf::from(path),
        source,
    }
}

pub(super) fn csv_row_number(row_offset: usize) -> usize {
    row_offset + 2
}
