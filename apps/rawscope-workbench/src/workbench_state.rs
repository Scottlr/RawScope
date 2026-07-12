//! Composition-root ownership for active truth and its operational projections.

use crate::{
    active_generation::ActiveWorkbenchState, degraded_state::DegradedStateStore, ui::ExportStatus,
    ui_projection_cache::UiProjectionCache,
};

#[derive(Debug, Default)]
pub(crate) struct WorkbenchState {
    pub(crate) active_generation: ActiveWorkbenchState,
    pub(crate) degraded: DegradedStateStore,
    pub(crate) ui_projection_cache: UiProjectionCache,
    pub(crate) export_status: ExportStatus,
    pub(crate) evidence_export_counter: u64,
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
