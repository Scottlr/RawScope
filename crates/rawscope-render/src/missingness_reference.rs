//! CPU reference missingness aggregation for local source-row inspection.

use rawscope_core::RowId;
use rawscope_data::LoadedSourceTable;

/// Missingness cell counts for one row bucket and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingnessCell {
    pub row_bucket: u32,
    pub column_index: u32,
    pub missing_count: u32,
    pub total_count: u32,
}

impl MissingnessCell {
    /// Returns the missing ratio for this cell.
    pub fn missing_ratio(self) -> f32 {
        if self.total_count == 0 {
            return 0.0;
        }

        self.missing_count as f32 / self.total_count as f32
    }
}

/// CPU reference missingness grid for local source rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingnessGrid {
    pub row_bucket_count: u32,
    pub column_count: u32,
    pub cells: Vec<MissingnessCell>,
}

impl MissingnessGrid {
    /// Looks up one grid cell by bucket and column.
    pub fn cell(&self, row_bucket: u32, column_index: u32) -> Option<&MissingnessCell> {
        if row_bucket >= self.row_bucket_count || column_index >= self.column_count {
            return None;
        }

        let column_count = usize::try_from(self.column_count).ok()?;
        let row_bucket_index = usize::try_from(row_bucket).ok()?;
        let column_index = usize::try_from(column_index).ok()?;
        let cell_index = row_bucket_index
            .checked_mul(column_count)?
            .checked_add(column_index)?;
        self.cells.get(cell_index)
    }
}

/// Selected row-bucket and column region in the missingness grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingnessSelection {
    pub row_bucket_start: u32,
    pub row_bucket_end_exclusive: u32,
    pub column_start: u32,
    pub column_end_exclusive: u32,
}

impl MissingnessSelection {
    /// Creates a non-empty missingness region selection.
    pub fn new(
        row_bucket_start: u32,
        row_bucket_end_exclusive: u32,
        column_start: u32,
        column_end_exclusive: u32,
    ) -> Self {
        assert!(
            row_bucket_end_exclusive > row_bucket_start,
            "missingness row bucket end must be greater than start"
        );
        assert!(
            column_end_exclusive > column_start,
            "missingness column end must be greater than start"
        );

        Self {
            row_bucket_start,
            row_bucket_end_exclusive,
            column_start,
            column_end_exclusive,
        }
    }

    fn contains_column(self, column_index: u32) -> bool {
        column_index >= self.column_start && column_index < self.column_end_exclusive
    }
}

/// Summary for one selected missingness region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingnessSelectionSummary {
    pub selected_missing_count: u64,
    pub selected_total_count: u64,
    pub selected_row_ids: Vec<RowId>,
    pub column_names: Vec<String>,
}

impl MissingnessSelectionSummary {
    /// Returns the selected missing ratio for this region.
    pub fn selected_missing_ratio(&self) -> f32 {
        if self.selected_total_count == 0 {
            return 0.0;
        }

        self.selected_missing_count as f32 / self.selected_total_count as f32
    }
}

/// Builds a deterministic CPU missingness grid from local source rows.
pub fn missingness_grid(
    source_table: &LoadedSourceTable,
    row_bucket_count: u32,
) -> MissingnessGrid {
    let normalized_row_bucket_count =
        normalized_row_bucket_count(row_bucket_count, source_table.rows.len());
    let column_count = u32::try_from(source_table.columns.len()).unwrap_or(u32::MAX);
    let total_cell_count = usize::try_from(normalized_row_bucket_count)
        .ok()
        .and_then(|row_bucket_count| row_bucket_count.checked_mul(source_table.columns.len()))
        .unwrap_or(0);
    let mut cells = Vec::with_capacity(total_cell_count);

    for row_bucket in 0..normalized_row_bucket_count {
        let bucket_row_indices = bucket_row_indices(
            source_table.rows.len(),
            normalized_row_bucket_count,
            row_bucket,
        );
        let selected_row_count = bucket_row_indices.len();

        for column_index in 0..column_count {
            let mut missing_count = 0;
            for row_index in bucket_row_indices.clone() {
                let row = &source_table.rows[row_index];
                let cell_value = row.values.get(column_index as usize).map(String::as_str);
                if cell_value.is_some_and(is_missing_value) {
                    missing_count += 1;
                }
            }

            cells.push(MissingnessCell {
                row_bucket,
                column_index,
                missing_count,
                total_count: selected_row_count as u32,
            });
        }
    }

    MissingnessGrid {
        row_bucket_count: normalized_row_bucket_count,
        column_count,
        cells,
    }
}

/// Summarizes one selected missingness region back to counts, columns, and row ids.
pub fn missingness_selection_summary(
    source_table: &LoadedSourceTable,
    row_bucket_count: u32,
    selection: MissingnessSelection,
) -> MissingnessSelectionSummary {
    let normalized_row_bucket_count =
        normalized_row_bucket_count(row_bucket_count, source_table.rows.len());
    let column_count = u32::try_from(source_table.columns.len()).unwrap_or(u32::MAX);
    let clamped_selection = clamp_selection(selection, normalized_row_bucket_count, column_count);
    let mut selected_missing_count = 0_u64;
    let mut selected_total_count = 0_u64;
    let mut selected_row_ids = Vec::new();

    for row_bucket in clamped_selection.row_bucket_start..clamped_selection.row_bucket_end_exclusive
    {
        let bucket_row_indices = bucket_row_indices(
            source_table.rows.len(),
            normalized_row_bucket_count,
            row_bucket,
        );
        let bucket_total_count = bucket_row_indices.len() as u64;

        for column_index in clamped_selection.column_start..clamped_selection.column_end_exclusive {
            selected_total_count += bucket_total_count;
            for row_index in bucket_row_indices.clone() {
                let row = &source_table.rows[row_index];
                let cell_value = row.values.get(column_index as usize).map(String::as_str);
                if cell_value.is_some_and(is_missing_value) {
                    selected_missing_count += 1;
                    selected_row_ids.push(row.row_id);
                }
            }
        }
    }

    selected_row_ids.sort_unstable_by_key(|row_id| row_id.0);
    selected_row_ids.dedup();

    let column_names = source_table
        .columns
        .iter()
        .enumerate()
        .filter_map(|(column_index, column)| {
            let column_index = u32::try_from(column_index).ok()?;
            clamped_selection
                .contains_column(column_index)
                .then(|| column.name.clone())
        })
        .collect();

    MissingnessSelectionSummary {
        selected_missing_count,
        selected_total_count,
        selected_row_ids,
        column_names,
    }
}

fn normalized_row_bucket_count(row_bucket_count: u32, row_count: usize) -> u32 {
    let row_count = u32::try_from(row_count).unwrap_or(u32::MAX);
    row_bucket_count.max(1).min(row_count.max(1))
}

fn clamp_selection(
    selection: MissingnessSelection,
    row_bucket_count: u32,
    column_count: u32,
) -> MissingnessSelection {
    let row_bucket_start = selection
        .row_bucket_start
        .min(row_bucket_count.saturating_sub(1));
    let row_bucket_end_exclusive = selection
        .row_bucket_end_exclusive
        .clamp(row_bucket_start + 1, row_bucket_count);
    let column_start = selection.column_start.min(column_count.saturating_sub(1));
    let column_end_exclusive = selection
        .column_end_exclusive
        .clamp(column_start + 1, column_count.max(1));

    MissingnessSelection::new(
        row_bucket_start,
        row_bucket_end_exclusive,
        column_start,
        column_end_exclusive,
    )
}

fn bucket_row_indices(
    row_count: usize,
    row_bucket_count: u32,
    row_bucket: u32,
) -> std::ops::Range<usize> {
    if row_count == 0 {
        return 0..0;
    }

    let row_bucket_start = (row_bucket as usize * row_count) / row_bucket_count as usize;
    let row_bucket_end = ((row_bucket as usize + 1) * row_count) / row_bucket_count as usize;
    row_bucket_start..row_bucket_end
}

fn is_missing_value(value: &str) -> bool {
    value.trim().is_empty()
}
