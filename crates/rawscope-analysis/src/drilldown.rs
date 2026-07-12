//! Checked disclosure for bounded selection drilldown results.

use rawscope_core::RowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceUnavailability {
    row_id: RowId,
}

impl SourceUnavailability {
    pub const fn new(row_id: RowId) -> Self {
        Self { row_id }
    }

    pub const fn row_id(self) -> RowId {
        self.row_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrilldownCompleteness {
    selected_count: u64,
    attempted_count: u64,
    returned_count: u64,
    sampled_due_to_limit: bool,
    unavailable: Vec<SourceUnavailability>,
}

impl DrilldownCompleteness {
    pub fn new(
        selected_count: u64,
        attempted_count: u64,
        returned_count: u64,
        sampled_due_to_limit: bool,
        unavailable: Vec<SourceUnavailability>,
    ) -> Self {
        Self {
            selected_count,
            attempted_count,
            returned_count,
            sampled_due_to_limit,
            unavailable,
        }
    }

    pub const fn selected_count(&self) -> u64 {
        self.selected_count
    }

    pub const fn attempted_count(&self) -> u64 {
        self.attempted_count
    }

    pub const fn returned_count(&self) -> u64 {
        self.returned_count
    }

    pub const fn sampled_due_to_limit(&self) -> bool {
        self.sampled_due_to_limit
    }

    pub fn unavailable(&self) -> &[SourceUnavailability] {
        &self.unavailable
    }
}
