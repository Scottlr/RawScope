//! Persistent, typed degraded state around the active workbench generation.

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DegradedKind {
    Dataset,
    Gpu,
    Render,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryAction {
    RetryDataset,
    RetryGpu,
    RetryRender,
    RetryExport,
    Dismiss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DegradedState {
    pub(crate) kind: DegradedKind,
    pub(crate) message: Arc<str>,
    pub(crate) generation: u64,
    pub(crate) recovery: RecoveryAction,
    pub(crate) dismissible: bool,
}

#[derive(Debug, Default)]
pub(crate) struct DegradedStateStore {
    current: Option<DegradedState>,
}

impl DegradedStateStore {
    pub(crate) fn current(&self) -> Option<&DegradedState> {
        self.current.as_ref()
    }

    pub(crate) fn record(&mut self, state: DegradedState) {
        self.current = Some(state);
    }

    pub(crate) fn clear_after_recovery(&mut self, kind: DegradedKind, generation: u64) -> bool {
        let matches = self
            .current
            .as_ref()
            .is_some_and(|state| state.kind == kind && state.generation == generation);
        if matches {
            self.current = None;
        }
        matches
    }

    pub(crate) fn dismiss(&mut self) -> bool {
        let dismissible = self.current.as_ref().is_some_and(|state| state.dismissible);
        if dismissible {
            self.current = None;
        }
        dismissible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(kind: DegradedKind, generation: u64, dismissible: bool) -> DegradedState {
        DegradedState {
            kind,
            message: Arc::from("GPU replacement failed"),
            generation,
            recovery: RecoveryAction::RetryGpu,
            dismissible,
        }
    }

    #[test]
    fn unrelated_success_does_not_clear_degraded_state() {
        let mut store = DegradedStateStore::default();
        store.record(state(DegradedKind::Gpu, 4, false));

        assert!(!store.clear_after_recovery(DegradedKind::Dataset, 4));
        assert_eq!(store.current().unwrap().kind, DegradedKind::Gpu);
    }

    #[test]
    fn only_matching_recovery_clears_the_error() {
        let mut store = DegradedStateStore::default();
        store.record(state(DegradedKind::Gpu, 4, false));

        assert!(!store.clear_after_recovery(DegradedKind::Gpu, 3));
        assert!(store.clear_after_recovery(DegradedKind::Gpu, 4));
        assert_eq!(store.current(), None);
    }

    #[test]
    fn dismiss_only_clears_dismissible_state() {
        let mut store = DegradedStateStore::default();
        store.record(state(DegradedKind::Gpu, 4, false));
        assert!(!store.dismiss());
        assert!(store.current().is_some());

        store.record(state(DegradedKind::Render, 5, true));
        assert!(store.dismiss());
        assert!(store.current().is_none());
    }
}
