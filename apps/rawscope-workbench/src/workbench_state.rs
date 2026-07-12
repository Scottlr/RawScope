//! Composition-root ownership for active truth and its operational projections.

use crate::{
    active_generation::ActiveWorkbenchState,
    degraded_state::DegradedStateStore,
    ui::{ExportStatus, WorkbenchSurface},
    ui_projection_cache::UiProjectionCache,
};
use rawscope_data::DatasetProfileId;
use rawscope_data::{LoadedSourceTable, SyntheticDatasetMetadata};
use rawscope_render::DatasetDiffSummary;

#[derive(Debug, Default)]
pub(crate) struct WorkbenchState {
    pub(crate) active_generation: ActiveWorkbenchState,
    pub(crate) degraded: DegradedStateStore,
    pub(crate) ui_projection_cache: UiProjectionCache,
    pub(crate) export_status: ExportStatus,
    pub(crate) evidence_export_counter: u64,
    pub(crate) dataset_diff_summary: Option<DatasetDiffSummary>,
    pub(crate) visible_surface: WorkbenchSurface,
    pub(crate) dataset_metadata: Option<SyntheticDatasetMetadata>,
    pub(crate) active_dataset_profile: Option<DatasetProfileId>,
    pub(crate) comparison_source_rows: Option<LoadedSourceTable>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_no_active_or_degraded_projection() {
        let state = WorkbenchState::default();

        assert!(state.active_generation.active().is_none());
        assert!(state.degraded.current().is_none());
        assert!(state.ui_projection_cache.key().is_none());
    }
}
