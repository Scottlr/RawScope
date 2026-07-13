//! Frame-owned scheduling for interactive scatter density refinement.

use std::time::Instant;

use rawscope_render::{
    DensityPresentationConfig, DensityReadbackPolicy, ResidentExactFieldUpdate, VisualFieldQuality,
    VisualFieldViewport,
};

use crate::app::{default_scatter_grid, WorkbenchApp};

const PREVIEW_REBIN_INTERVAL_MS: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InteractiveDensityPhase {
    SettledExact,
    Reprojecting,
    PreviewInFlight,
    FinalRefinePending,
    FinalRefineInFlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InteractiveDensityConfig {
    pub(crate) preview_rebin_interval_ms: u64,
}

impl Default for InteractiveDensityConfig {
    fn default() -> Self {
        Self {
            preview_rebin_interval_ms: PREVIEW_REBIN_INTERVAL_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScheduledDensityWork {
    Preview { revision: u64 },
    Exact { revision: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingExactReadback {
    pub(crate) work: ScheduledDensityWork,
}

pub(crate) struct RenderSchedule {
    phase: InteractiveDensityPhase,
    config: InteractiveDensityConfig,
    latest_viewport_revision: u64,
    source_viewport_revision: u64,
    dispatched_viewport_revision: Option<u64>,
    last_preview_dispatch_at_ms: Option<u64>,
    started_at: Instant,
    settled_summary_refreshes: u64,
}

impl Default for RenderSchedule {
    fn default() -> Self {
        Self {
            phase: InteractiveDensityPhase::SettledExact,
            config: InteractiveDensityConfig::default(),
            latest_viewport_revision: 0,
            source_viewport_revision: 0,
            dispatched_viewport_revision: None,
            last_preview_dispatch_at_ms: None,
            started_at: Instant::now(),
            settled_summary_refreshes: 0,
        }
    }
}

impl RenderSchedule {
    pub(crate) fn gesture_started(&mut self) {
        if self.phase == InteractiveDensityPhase::SettledExact {
            self.phase = InteractiveDensityPhase::Reprojecting;
        }
    }
    pub(crate) fn viewport_changed(&mut self) -> u64 {
        self.latest_viewport_revision += 1;
        if self.phase == InteractiveDensityPhase::SettledExact {
            self.phase = InteractiveDensityPhase::Reprojecting;
        }
        self.latest_viewport_revision
    }
    pub(crate) fn gesture_released(&mut self) {
        if self.phase != InteractiveDensityPhase::SettledExact {
            self.phase = InteractiveDensityPhase::FinalRefinePending;
            self.dispatched_viewport_revision = None;
        }
    }
    pub(crate) fn is_refining(&self) -> bool {
        self.phase != InteractiveDensityPhase::SettledExact
    }
    pub(crate) fn elapsed_ms(&self) -> u64 {
        self.started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }

    pub(crate) fn next_work(&mut self, now_ms: u64) -> Option<ScheduledDensityWork> {
        match self.phase {
            InteractiveDensityPhase::FinalRefinePending => {
                let revision = self.latest_viewport_revision;
                self.phase = InteractiveDensityPhase::FinalRefineInFlight;
                self.dispatched_viewport_revision = Some(revision);
                Some(ScheduledDensityWork::Exact { revision })
            }
            InteractiveDensityPhase::Reprojecting if self.preview_is_due(now_ms) => {
                let revision = self.latest_viewport_revision;
                self.phase = InteractiveDensityPhase::PreviewInFlight;
                self.dispatched_viewport_revision = Some(revision);
                self.last_preview_dispatch_at_ms = Some(now_ms);
                Some(ScheduledDensityWork::Preview { revision })
            }
            _ => None,
        }
    }

    pub(crate) fn work_completed(&mut self, work: ScheduledDensityWork) -> bool {
        match work {
            ScheduledDensityWork::Preview { revision }
                if self.phase == InteractiveDensityPhase::PreviewInFlight =>
            {
                self.source_viewport_revision = revision;
                self.dispatched_viewport_revision = None;
                self.phase = InteractiveDensityPhase::Reprojecting;
                false
            }
            ScheduledDensityWork::Exact { revision }
                if self.phase == InteractiveDensityPhase::FinalRefineInFlight
                    && revision == self.latest_viewport_revision =>
            {
                self.source_viewport_revision = revision;
                self.dispatched_viewport_revision = None;
                self.phase = InteractiveDensityPhase::SettledExact;
                self.settled_summary_refreshes += 1;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn work_failed(&mut self, work: ScheduledDensityWork) {
        self.dispatched_viewport_revision = None;
        self.phase = match work {
            ScheduledDensityWork::Preview { .. } => InteractiveDensityPhase::Reprojecting,
            ScheduledDensityWork::Exact { .. } => InteractiveDensityPhase::FinalRefinePending,
        };
    }

    pub(crate) fn exact_field_settled(&mut self) {
        self.phase = InteractiveDensityPhase::SettledExact;
        self.source_viewport_revision = self.latest_viewport_revision;
        self.dispatched_viewport_revision = None;
    }

    pub(crate) fn request_exact_refine(&mut self) {
        self.latest_viewport_revision += 1;
        self.phase = InteractiveDensityPhase::FinalRefinePending;
        self.dispatched_viewport_revision = None;
    }

    pub(crate) fn settled_revision(&self) -> u64 {
        self.source_viewport_revision
    }

    fn preview_is_due(&self, now_ms: u64) -> bool {
        self.last_preview_dispatch_at_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= self.config.preview_rebin_interval_ms)
    }
}

impl WorkbenchApp {
    pub(crate) fn begin_interactive_density(&mut self) {
        self.cancel_visual_transition();
        self.render_schedule.gesture_started();
    }

    pub(crate) fn interactive_viewport_changed(&mut self) {
        let Some(viewport) = self.scatter.viewport else {
            return;
        };
        self.render_schedule.viewport_changed();
        if let Some(renderer) = self.scatter.density_renderer.as_mut() {
            renderer.cancel_full_readback();
        }
        self.scatter.pending_settled_context_readback = false;
        self.invalidate_scatter_inspection();
        self.invalidate_scatter_point_reveal();
        self.reproject_point_reveal_viewport();
        self.scatter.difference_baseline_dirty = true;
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.density_renderer.as_mut())
        {
            renderer.set_display_viewport(gpu.queue(), viewport.x_range(), viewport.y_range());
        }
        if let Some(renderer) = self.scatter.difference_renderer.as_mut() {
            renderer.set_display_viewport(viewport.x_range(), viewport.y_range());
        }
        self.request_redraw();
    }

    pub(crate) fn finish_interactive_density(&mut self) {
        self.render_schedule.gesture_released();
        self.request_redraw();
    }

    pub(crate) fn prepare_scheduled_density(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now_ms = self.render_schedule.elapsed_ms();
        if self.scatter.viewport.is_none()
            || self.gpu.is_none()
            || self.scatter.density_renderer.is_none()
        {
            return Ok(());
        }

        if self.scatter.pending_settled_context_readback {
            let poll_result = {
                let gpu = self.gpu.as_ref().expect("GPU checked above");
                let renderer = self
                    .scatter
                    .density_renderer
                    .as_mut()
                    .expect("density renderer checked above");
                renderer.poll_full_readback(gpu.device())
            };
            match poll_result {
                Ok(None) => {
                    let still_pending = self
                        .scatter
                        .density_renderer
                        .as_ref()
                        .is_some_and(|renderer| renderer.has_pending_full_readback());
                    if !still_pending {
                        self.scatter.pending_settled_context_readback = false;
                    } else {
                        self.request_redraw();
                    }
                    return Ok(());
                }
                Ok(Some(count_grid)) => {
                    self.scatter.pending_settled_context_readback = false;
                    self.publish_scatter_density_context(count_grid)?;
                    self.rebuild_scatter_inspection_cache();
                    self.update_window_title();
                    self.request_redraw();
                    return Ok(());
                }
                Err(err) => {
                    self.scatter.pending_settled_context_readback = false;
                    return Err(err.into());
                }
            }
        }

        if let Some(pending) = self.scatter.pending_exact_readback {
            let poll_result = {
                let gpu = self.gpu.as_ref().expect("GPU checked above");
                let renderer = self
                    .scatter
                    .density_renderer
                    .as_mut()
                    .expect("density renderer checked above");
                renderer.poll_full_readback(gpu.device())
            };
            match poll_result {
                Ok(None) => {
                    let still_pending = self
                        .scatter
                        .density_renderer
                        .as_ref()
                        .is_some_and(|renderer| renderer.has_pending_full_readback());
                    if !still_pending {
                        self.scatter.pending_exact_readback = None;
                        self.render_schedule.work_failed(pending.work);
                        return Ok(());
                    }
                    self.request_redraw();
                    return Ok(());
                }
                Ok(Some(count_grid)) => {
                    self.scatter.pending_exact_readback = None;
                    if self.render_schedule.work_completed(pending.work) {
                        self.publish_scatter_density_context(count_grid)?;
                        self.rebuild_scatter_inspection_cache();
                        self.invalidate_scatter_point_reveal();
                        self.update_window_title();
                        self.begin_visual_transition(
                            rawscope_render::TransitionKind::DensityRefresh,
                        );
                    }
                    return Ok(());
                }
                Err(err) => {
                    self.scatter.pending_exact_readback = None;
                    self.render_schedule.work_failed(pending.work);
                    return Err(err.into());
                }
            }
        }
        let Some(work) = self.render_schedule.next_work(now_ms) else {
            return Ok(());
        };
        let viewport = self.scatter.viewport.expect("viewport checked above");
        let (quality, readback, revision) = match work {
            ScheduledDensityWork::Preview { revision } => (
                VisualFieldQuality::Preview,
                DensityReadbackPolicy::None,
                revision,
            ),
            ScheduledDensityWork::Exact { revision } => (
                VisualFieldQuality::Exact,
                DensityReadbackPolicy::None,
                revision,
            ),
        };
        let gpu = self.gpu.as_ref().expect("GPU checked above");
        let grid = default_scatter_grid(gpu, quality)?;
        let grid_width = grid.width();
        let grid_height = grid.height();
        let config = DensityPresentationConfig::new(
            viewport.x_range(),
            viewport.y_range(),
            grid_width,
            grid_height,
        )
        .with_encoding(self.scatter.density_encoding)
        .with_presentation(self.scatter.density_presentation)
        .with_relief(self.scatter.relief_config);
        let field = VisualFieldViewport {
            x_range: viewport.x_range(),
            y_range: viewport.y_range(),
            grid_width,
            grid_height,
            viewport_revision: revision,
            quality,
        };
        let update_result = {
            let gpu = self.gpu.as_ref().expect("GPU checked above");
            let renderer = self
                .scatter
                .density_renderer
                .as_mut()
                .expect("density renderer checked above");
            renderer.update_density_for_field(
                gpu.device(),
                gpu.queue(),
                ResidentExactFieldUpdate { config, readback },
                field,
            )
        };
        let stats = match update_result {
            Ok(stats) => stats,
            Err(err) => {
                self.render_schedule.work_failed(work);
                return Err(err.into());
            }
        };
        self.scatter.render_stats = Some(stats);
        if let Some(renderer) = self.scatter.difference_renderer.as_mut() {
            let active_total = self.scatter_filters.cohort_snapshot.as_ref().map_or_else(
                || {
                    self.scatter_filters
                        .evaluation
                        .as_ref()
                        .map_or(self.scatter.points.len() as u64, |evaluation| {
                            evaluation.included_count as u64
                        })
                },
                |snapshot| snapshot.included_row_count(),
            );
            let gpu = self.gpu.as_ref().expect("GPU checked above");
            self.scatter.difference_stats = Some(renderer.update_fields(
                gpu.device(),
                gpu.queue(),
                config,
                self.scatter.difference_baseline_dirty,
                active_total,
                revision,
            )?);
            self.scatter.difference_baseline_dirty = false;
        }

        if matches!(work, ScheduledDensityWork::Exact { .. }) {
            self.scatter.pending_settled_context_readback = false;
            let begin_result = {
                let gpu = self.gpu.as_ref().expect("GPU checked above");
                let renderer = self
                    .scatter
                    .density_renderer
                    .as_mut()
                    .expect("density renderer checked above");
                renderer.begin_full_readback(gpu.device(), gpu.queue())
            };
            if let Err(err) = begin_result {
                self.render_schedule.work_failed(work);
                return Err(err.into());
            }
            self.scatter.pending_exact_readback = Some(PendingExactReadback { work });
            self.request_redraw();
            return Ok(());
        }

        let settled = self.render_schedule.work_completed(work);
        if settled {
            self.refresh_scatter_marginal_summary();
            self.rebuild_scatter_inspection_cache();
            self.invalidate_scatter_point_reveal();
            self.update_window_title();
            self.begin_visual_transition(rawscope_render::TransitionKind::DensityRefresh);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_move_only_updates_latest_viewport_revision() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        assert_eq!(schedule.viewport_changed(), 1);
        assert_eq!(schedule.viewport_changed(), 2);
        assert_eq!(schedule.phase, InteractiveDensityPhase::Reprojecting);
    }
    #[test]
    fn preview_dispatch_coalesces_moves_and_allows_one_in_flight() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        schedule.viewport_changed();
        schedule.viewport_changed();
        assert_eq!(
            schedule.next_work(50),
            Some(ScheduledDensityWork::Preview { revision: 2 })
        );
        assert_eq!(schedule.next_work(60), None);
    }
    #[test]
    fn preview_completion_cannot_replace_newer_final_revision() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        schedule.viewport_changed();
        let preview = schedule.next_work(50).unwrap();
        schedule.viewport_changed();
        schedule.gesture_released();
        assert!(!schedule.work_completed(preview));
        assert_eq!(schedule.phase, InteractiveDensityPhase::FinalRefinePending);
    }
    #[test]
    fn gesture_release_requests_one_exact_density_refine() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        schedule.viewport_changed();
        schedule.gesture_released();
        assert_eq!(
            schedule.next_work(1),
            Some(ScheduledDensityWork::Exact { revision: 1 })
        );
        assert_eq!(schedule.next_work(2), None);
    }
    #[test]
    fn settled_refine_refreshes_cpu_summaries_once() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        schedule.viewport_changed();
        schedule.gesture_released();
        let work = schedule.next_work(1).unwrap();
        assert!(schedule.work_completed(work));
        assert_eq!(schedule.settled_summary_refreshes, 1);
        assert!(!schedule.work_completed(work));
        assert_eq!(schedule.settled_summary_refreshes, 1);
    }

    #[test]
    fn failed_work_returns_to_a_retryable_phase() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        schedule.viewport_changed();
        let preview = schedule.next_work(50).unwrap();
        schedule.work_failed(preview);
        assert_eq!(schedule.phase, InteractiveDensityPhase::Reprojecting);

        schedule.gesture_released();
        let exact = schedule.next_work(51).unwrap();
        schedule.work_failed(exact);
        assert_eq!(schedule.phase, InteractiveDensityPhase::FinalRefinePending);
    }

    #[test]
    fn direct_exact_refresh_cancels_interactive_work() {
        let mut schedule = RenderSchedule::default();
        schedule.gesture_started();
        schedule.viewport_changed();
        schedule.next_work(50).unwrap();

        schedule.exact_field_settled();

        assert_eq!(schedule.phase, InteractiveDensityPhase::SettledExact);
        assert_eq!(schedule.dispatched_viewport_revision, None);
        assert_eq!(schedule.source_viewport_revision, 1);
    }

    #[test]
    fn filter_revision_requests_one_exact_refine() {
        let mut schedule = RenderSchedule::default();

        schedule.request_exact_refine();

        assert_eq!(
            schedule.next_work(0),
            Some(ScheduledDensityWork::Exact { revision: 1 })
        );
        assert_eq!(schedule.next_work(1), None);
    }
}
