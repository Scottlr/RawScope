//! Workbench-owned lifecycle for bounded visual transitions.

use std::time::Instant;

use rawscope_render::{
    ease_out_cubic, transition_progress, ScatterDensityMode, TransitionKind, VisualTransitionConfig,
};

use crate::app::WorkbenchApp;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum WorkbenchTransitionState {
    #[default]
    Idle,
    Running {
        kind: TransitionKind,
        started_at: Instant,
        from_revision: u64,
        to_revision: u64,
        semantic_modes: Option<(ScatterDensityMode, ScatterDensityMode)>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TransitionFrame {
    pub(crate) alpha: f32,
    pub(crate) semantic_modes: Option<(ScatterDensityMode, ScatterDensityMode)>,
    pub(crate) running: bool,
}

impl WorkbenchTransitionState {
    fn begin(
        &mut self,
        config: VisualTransitionConfig,
        kind: TransitionKind,
        from_revision: u64,
        to_revision: u64,
        semantic_modes: Option<(ScatterDensityMode, ScatterDensityMode)>,
    ) {
        if config.reduced_motion || config.duration_ms == 0 {
            *self = Self::Idle;
        } else {
            *self = Self::Running {
                kind,
                started_at: Instant::now(),
                from_revision,
                to_revision,
                semantic_modes,
            };
        }
    }

    fn frame(&mut self, config: VisualTransitionConfig) -> TransitionFrame {
        let Self::Running {
            kind,
            started_at,
            from_revision,
            to_revision,
            semantic_modes,
        } = *self
        else {
            return TransitionFrame {
                alpha: 1.0,
                semantic_modes: None,
                running: false,
            };
        };
        let _transition_identity = (kind, from_revision, to_revision);
        let elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let progress = transition_progress(elapsed_ms, config);
        let alpha = ease_out_cubic(progress);
        if progress.0 >= 1.0 {
            *self = Self::Idle;
            TransitionFrame {
                alpha: 1.0,
                semantic_modes: None,
                running: false,
            }
        } else {
            TransitionFrame {
                alpha,
                semantic_modes,
                running: true,
            }
        }
    }

    fn cancel(&mut self) {
        *self = Self::Idle;
    }

    #[cfg(test)]
    fn is_running(self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

impl WorkbenchApp {
    pub(crate) fn begin_visual_transition(&mut self, kind: TransitionKind) {
        let from_revision = self.render_schedule.settled_revision();
        self.visual_transition.state.begin(
            self.visual_transition.config,
            kind,
            from_revision,
            from_revision.saturating_add(1),
            None,
        );
        self.request_redraw();
    }

    pub(crate) fn begin_density_mode_transition(
        &mut self,
        from: ScatterDensityMode,
        to: ScatterDensityMode,
    ) {
        let revision = self.render_schedule.settled_revision();
        self.visual_transition.state.begin(
            self.visual_transition.config,
            TransitionKind::DifferenceModeChange,
            revision,
            revision,
            Some((from, to)),
        );
        self.request_redraw();
    }

    pub(crate) fn cancel_visual_transition(&mut self) {
        self.visual_transition.state.cancel();
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.density_renderer.as_mut())
        {
            renderer.set_transition_progress(gpu.queue(), 1.0);
        }
    }

    pub(crate) fn set_reduced_motion(&mut self, reduced_motion: bool) {
        if self.visual_transition.config.reduced_motion == reduced_motion {
            return;
        }
        self.visual_transition.config.reduced_motion = reduced_motion;
        if reduced_motion {
            self.cancel_visual_transition();
        }
        self.request_redraw();
    }

    pub(crate) fn visual_transition_frame(&mut self) -> TransitionFrame {
        self.visual_transition
            .state
            .frame(self.visual_transition.config)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WorkbenchVisualTransition {
    pub(crate) config: VisualTransitionConfig,
    pub(crate) state: WorkbenchTransitionState,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn active_gesture_cancels_running_transition() {
        let mut app = WorkbenchApp::default();
        app.begin_visual_transition(TransitionKind::DensityRefresh);
        assert!(app.visual_transition.state.is_running());
        app.begin_interactive_density();
        assert!(!app.visual_transition.state.is_running());
    }

    #[test]
    fn completed_transition_releases_previous_revision() {
        let mut state = WorkbenchTransitionState::Running {
            kind: TransitionKind::DensityRefresh,
            started_at: Instant::now() - Duration::from_millis(200),
            from_revision: 4,
            to_revision: 5,
            semantic_modes: None,
        };
        let frame = state.frame(VisualTransitionConfig::default());
        assert!(!frame.running);
        assert!(!state.is_running());
    }

    #[test]
    fn idle_transition_requests_no_redraw() {
        let mut state = WorkbenchTransitionState::default();
        assert!(!state.frame(VisualTransitionConfig::default()).running);
    }
}
