//! CPU-backed row drilldown for finalized scatter and timeline selections.

use rawscope_analysis::drilldown::{DrilldownCompleteness, SourceUnavailability};
use rawscope_core::RowId;
use rawscope_data::{
    FilterMask, LoadedSourceRow, LoadedSourceTable, ScatterPointKind, ScatterPointRecord,
    TimelineEventKind, TimelineEventRecord,
};

use crate::evidence_sample::{insert_lowest_row_id_sample, RowIdSample};
use crate::{ScatterBrushSelection, SelectionSnapshot, TimelineBrushSelection};

const DEFAULT_MAX_DRILLDOWN_ROWS: usize = 100;
const SCATTER_FALLBACK_COLUMN_NAMES: [&str; 4] = ["row_id", "x", "y", "kind"];
const TIMELINE_FALLBACK_COLUMN_NAMES: [&str; 5] = ["row_id", "timestamp", "lane", "value", "kind"];

/// One named drilldown column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrilldownColumn {
    pub name: String,
}

pub fn scatter_selection_drilldown_masked(
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    selection: ScatterBrushSelection,
    source_rows: Option<&LoadedSourceTable>,
    config: DrilldownConfig,
) -> Result<SelectionDrilldown, crate::MaskAlignmentError> {
    crate::MaskAlignmentError::require(points.len(), mask.len())?;
    let columns = source_rows
        .map(source_columns)
        .unwrap_or_else(scatter_fallback_columns);
    let mut selected_row_count = 0;
    let mut rows = Vec::new();
    for (point, included) in points.iter().zip(mask.as_gpu_u32_slice()) {
        if *included == 0 || !selection.contains_point(point) {
            continue;
        }
        selected_row_count += 1;
        let row = match source_rows {
            Some(table) => source_row(table, point.row_id),
            None => Some(scatter_fallback_row(point)),
        };
        if let Some(row) = row {
            insert_lowest_row_id_sample(&mut rows, row, config.max_rows);
        }
    }
    let displayed_row_count = rows.len();
    let rows_are_sampled = selected_row_count > displayed_row_count;
    Ok(SelectionDrilldown {
        selected_row_count,
        displayed_row_count,
        rows_are_sampled,
        columns,
        rows,
        completeness: DrilldownCompleteness::new(
            selected_row_count as u64,
            selected_row_count as u64,
            displayed_row_count as u64,
            rows_are_sampled,
            Vec::new(),
        ),
    })
}

/// Builds scatter drilldown rows from an already-finalized immutable snapshot.
pub fn scatter_selection_drilldown_snapshot(
    points: &[ScatterPointRecord],
    snapshot: &SelectionSnapshot,
    source_rows: Option<&LoadedSourceTable>,
    config: DrilldownConfig,
) -> SelectionDrilldown {
    let columns = source_rows
        .map(source_columns)
        .unwrap_or_else(scatter_fallback_columns);
    let mut rows = Vec::new();
    let mut unavailable = Vec::new();
    for point in points {
        if snapshot.row_ids().binary_search(&point.row_id).is_err() {
            continue;
        }
        let row = match source_rows {
            Some(table) => source_row(table, point.row_id).or_else(|| {
                unavailable.push(SourceUnavailability::new(point.row_id));
                None
            }),
            None => Some(scatter_fallback_row(point)),
        };
        if let Some(row) = row {
            insert_lowest_row_id_sample(&mut rows, row, config.max_rows);
        }
    }
    let displayed_row_count = rows.len();
    let rows_are_sampled = snapshot.selected_count() > displayed_row_count;
    SelectionDrilldown {
        selected_row_count: snapshot.selected_count(),
        displayed_row_count,
        rows_are_sampled,
        columns,
        rows,
        completeness: DrilldownCompleteness::new(
            snapshot.selected_count() as u64,
            snapshot.selected_count() as u64,
            displayed_row_count as u64,
            rows_are_sampled,
            unavailable,
        ),
    }
}

/// One selected drilldown row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrilldownRow {
    pub row_id: RowId,
    pub values: Vec<String>,
}

impl RowIdSample for DrilldownRow {
    fn row_id(&self) -> RowId {
        self.row_id
    }
}

/// Bounded drilldown sampling configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrilldownConfig {
    pub max_rows: usize,
}

impl Default for DrilldownConfig {
    fn default() -> Self {
        Self {
            max_rows: DEFAULT_MAX_DRILLDOWN_ROWS,
        }
    }
}

/// CPU-backed selected-row drilldown prepared for UI and later export surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionDrilldown {
    pub selected_row_count: usize,
    pub displayed_row_count: usize,
    pub rows_are_sampled: bool,
    pub columns: Vec<DrilldownColumn>,
    pub rows: Vec<DrilldownRow>,
    pub completeness: DrilldownCompleteness,
}

/// Builds deterministic drilldown rows from a finalized scatter selection.
pub fn scatter_selection_drilldown(
    points: &[ScatterPointRecord],
    selection: ScatterBrushSelection,
    source_rows: Option<&LoadedSourceTable>,
    config: DrilldownConfig,
) -> SelectionDrilldown {
    let columns = source_rows
        .map(source_columns)
        .unwrap_or_else(scatter_fallback_columns);

    build_selection_drilldown(
        points,
        columns,
        config,
        |point| selection.contains_point(point),
        |point| match source_rows {
            Some(table) => source_row(table, point.row_id),
            None => Some(scatter_fallback_row(point)),
        },
    )
}

/// Builds deterministic drilldown rows from a finalized timeline selection.
pub fn timeline_selection_drilldown(
    events: &[TimelineEventRecord],
    selection: TimelineBrushSelection,
    source_rows: Option<&LoadedSourceTable>,
    config: DrilldownConfig,
) -> SelectionDrilldown {
    let columns = source_rows
        .map(source_columns)
        .unwrap_or_else(timeline_fallback_columns);

    build_selection_drilldown(
        events,
        columns,
        config,
        |event| selection.contains_event(event),
        |event| match source_rows {
            Some(table) => source_row(table, event.row_id),
            None => Some(timeline_fallback_row(event)),
        },
    )
}

/// Builds timeline drilldown rows from an already-finalized immutable snapshot.
pub fn timeline_selection_drilldown_snapshot(
    events: &[TimelineEventRecord],
    snapshot: &SelectionSnapshot,
    source_rows: Option<&LoadedSourceTable>,
    config: DrilldownConfig,
) -> SelectionDrilldown {
    let columns = source_rows
        .map(source_columns)
        .unwrap_or_else(timeline_fallback_columns);
    let mut rows = Vec::new();
    let mut unavailable = Vec::new();
    for event in events {
        if snapshot.row_ids().binary_search(&event.row_id).is_err() {
            continue;
        }
        let row = match source_rows {
            Some(table) => source_row(table, event.row_id).or_else(|| {
                unavailable.push(SourceUnavailability::new(event.row_id));
                None
            }),
            None => Some(timeline_fallback_row(event)),
        };
        if let Some(row) = row {
            insert_lowest_row_id_sample(&mut rows, row, config.max_rows);
        }
    }
    let displayed_row_count = rows.len();
    let rows_are_sampled = snapshot.selected_count() > displayed_row_count;
    SelectionDrilldown {
        selected_row_count: snapshot.selected_count(),
        displayed_row_count,
        rows_are_sampled,
        columns,
        rows,
        completeness: DrilldownCompleteness::new(
            snapshot.selected_count() as u64,
            snapshot.selected_count() as u64,
            displayed_row_count as u64,
            rows_are_sampled,
            unavailable,
        ),
    }
}

fn build_selection_drilldown<T>(
    records: &[T],
    columns: Vec<DrilldownColumn>,
    config: DrilldownConfig,
    record_is_selected: impl Fn(&T) -> bool,
    build_row: impl Fn(&T) -> Option<DrilldownRow>,
) -> SelectionDrilldown {
    let mut selected_row_count = 0;
    let mut rows = Vec::new();

    for record in records {
        if !record_is_selected(record) {
            continue;
        }

        selected_row_count += 1;
        if let Some(row) = build_row(record) {
            insert_lowest_row_id_sample(&mut rows, row, config.max_rows);
        }
    }

    let rows_are_sampled = selected_row_count > config.max_rows;
    let displayed_row_count = rows.len();

    SelectionDrilldown {
        selected_row_count,
        displayed_row_count,
        rows_are_sampled,
        columns,
        rows,
        completeness: DrilldownCompleteness::new(
            selected_row_count as u64,
            selected_row_count as u64,
            displayed_row_count as u64,
            rows_are_sampled,
            Vec::new(),
        ),
    }
}

fn source_columns(table: &LoadedSourceTable) -> Vec<DrilldownColumn> {
    table
        .column_names()
        .map(|name| DrilldownColumn {
            name: name.to_string(),
        })
        .collect()
}

fn scatter_fallback_columns() -> Vec<DrilldownColumn> {
    SCATTER_FALLBACK_COLUMN_NAMES
        .into_iter()
        .map(|name| DrilldownColumn {
            name: name.to_string(),
        })
        .collect()
}

fn timeline_fallback_columns() -> Vec<DrilldownColumn> {
    TIMELINE_FALLBACK_COLUMN_NAMES
        .into_iter()
        .map(|name| DrilldownColumn {
            name: name.to_string(),
        })
        .collect()
}

fn source_row(table: &LoadedSourceTable, row_id: RowId) -> Option<DrilldownRow> {
    table.row(row_id).map(DrilldownRow::from)
}

fn scatter_fallback_row(point: &ScatterPointRecord) -> DrilldownRow {
    DrilldownRow {
        row_id: point.row_id,
        values: vec![
            point.row_id.0.to_string(),
            format_f32(point.x),
            format_f32(point.y),
            scatter_point_kind_label(point.kind).to_string(),
        ],
    }
}

fn timeline_fallback_row(event: &TimelineEventRecord) -> DrilldownRow {
    DrilldownRow {
        row_id: event.row_id,
        values: vec![
            event.row_id.0.to_string(),
            event.timestamp.to_string(),
            event.lane.to_string(),
            format_f32(event.value),
            timeline_event_kind_label(event.kind).to_string(),
        ],
    }
}

fn format_f32(value: f32) -> String {
    format!("{value:.6}")
}

fn scatter_point_kind_label(kind: ScatterPointKind) -> &'static str {
    match kind {
        ScatterPointKind::Synthetic(category) => match category {
            rawscope_data::SyntheticPointCategory::Cluster => "cluster",
            rawscope_data::SyntheticPointCategory::Background => "background",
            rawscope_data::SyntheticPointCategory::Outlier => "outlier",
        },
        ScatterPointKind::Unclassified => "unclassified",
    }
}

fn timeline_event_kind_label(kind: TimelineEventKind) -> &'static str {
    match kind {
        TimelineEventKind::Synthetic(event_type) => match event_type {
            rawscope_data::SyntheticEventType::Background => "background",
            rawscope_data::SyntheticEventType::Spike => "spike",
            rawscope_data::SyntheticEventType::StaleLane => "stale_lane",
            rawscope_data::SyntheticEventType::HighValueBand => "high_value_band",
        },
        TimelineEventKind::Unclassified => "unclassified",
    }
}

impl From<&LoadedSourceRow> for DrilldownRow {
    fn from(row: &LoadedSourceRow) -> Self {
        Self {
            row_id: row.row_id,
            values: row.values.clone(),
        }
    }
}
