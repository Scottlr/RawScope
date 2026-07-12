//! Startup replacement state that never discards a previously ready view.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct StartupRequestId(u64);

impl StartupRequestId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StartupPhase {
    Idle,
    ResolvingSession {
        request: StartupRequestId,
    },
    InitializingGpu {
        request: StartupRequestId,
    },
    LoadingDataset {
        request: StartupRequestId,
    },
    PreparingView {
        request: StartupRequestId,
    },
    Ready,
    Failed {
        request: StartupRequestId,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupLifecycle<T> {
    active: Option<T>,
    phase: StartupPhase,
}

impl<T> Default for StartupLifecycle<T> {
    fn default() -> Self {
        Self {
            active: None,
            phase: StartupPhase::Idle,
        }
    }
}

impl<T> StartupLifecycle<T> {
    pub(crate) fn active(&self) -> Option<&T> {
        self.active.as_ref()
    }

    pub(crate) const fn phase(&self) -> &StartupPhase {
        &self.phase
    }

    pub(crate) fn begin(&mut self, request: StartupRequestId) {
        self.phase = StartupPhase::ResolvingSession { request };
    }

    pub(crate) fn gpu_initializing(&mut self, request: StartupRequestId) -> bool {
        self.advance(request, StartupPhase::InitializingGpu { request })
    }

    pub(crate) fn dataset_loading(&mut self, request: StartupRequestId) -> bool {
        self.advance(request, StartupPhase::LoadingDataset { request })
    }

    pub(crate) fn view_preparing(&mut self, request: StartupRequestId) -> bool {
        self.advance(request, StartupPhase::PreparingView { request })
    }

    pub(crate) fn publish_ready(&mut self, request: StartupRequestId, active: T) -> bool {
        if !self.matches(request) {
            return false;
        }
        self.active = Some(active);
        self.phase = StartupPhase::Ready;
        true
    }

    pub(crate) fn fail(&mut self, request: StartupRequestId, message: String) -> bool {
        if !self.matches(request) {
            return false;
        }
        self.phase = StartupPhase::Failed { request, message };
        true
    }

    fn advance(&mut self, request: StartupRequestId, phase: StartupPhase) -> bool {
        if !self.matches(request) {
            return false;
        }
        self.phase = phase;
        true
    }

    fn matches(&self, request: StartupRequestId) -> bool {
        matches!(
            self.phase,
            StartupPhase::ResolvingSession { request: current }
                | StartupPhase::InitializingGpu { request: current }
                | StartupPhase::LoadingDataset { request: current }
                | StartupPhase::PreparingView { request: current }
                if current == request
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_failure_preserves_active_ready_state() {
        let mut lifecycle = StartupLifecycle::default();
        let first = StartupRequestId::new(1);
        lifecycle.begin(first);
        assert!(lifecycle.publish_ready(first, "old"));

        let replacement = StartupRequestId::new(2);
        lifecycle.begin(replacement);
        assert!(lifecycle.gpu_initializing(replacement));
        assert!(lifecycle.fail(replacement, "GPU unavailable".to_string()));
        assert_eq!(lifecycle.active(), Some(&"old"));
        assert!(matches!(lifecycle.phase(), StartupPhase::Failed { .. }));
    }

    #[test]
    fn stale_completion_cannot_replace_current_request() {
        let mut lifecycle = StartupLifecycle::default();
        let first = StartupRequestId::new(1);
        let second = StartupRequestId::new(2);
        lifecycle.begin(first);
        lifecycle.begin(second);
        assert!(!lifecycle.publish_ready(first, "stale"));
        assert!(lifecycle.view_preparing(second));
        assert!(lifecycle.publish_ready(second, "current"));
        assert_eq!(lifecycle.active(), Some(&"current"));
    }
}
