//! Settled point-reveal planning and GPU overlay coordination.

use std::sync::{Arc, Mutex};

use rawscope_analysis::visual_field::{
    build_point_reveal_plan_from_points, semantic_zoom_frame, PointRevealPlan,
    PointRevealPlanError, PointRevealViewport, SemanticZoomFrame, SemanticZoomPolicy,
};
use rawscope_core::RowId;
use rawscope_data::DatasetGeneration;
use rawscope_gpu::GpuContext;
use rawscope_render::{
    PointRevealConfig, PointRevealMode, PointRevealStats, ScatterPointRenderer,
    VisualFieldViewGeneration,
};
use tracing::{error, warn};

use crate::{
    app::WorkbenchApp,
    job_coordinator::{
        CancellationToken, JobHandle, JobOutcome, WorkbenchJobGeneration, WorkbenchJobKind,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointRevealJobIdentity {
    pub(crate) dataset_generation: DatasetGeneration,
    pub(crate) cohort_generation: rawscope_analysis::cohort::CohortGeneration,
    pub(crate) view_generation: VisualFieldViewGeneration,
    pub(crate) viewport_revision: u64,
}

pub(crate) fn point_reveal_identity_matches(
    expected: PointRevealJobIdentity,
    current: PointRevealJobIdentity,
) -> bool {
    expected == current
}

pub(crate) type PointRevealJobResult =
    Arc<Mutex<Option<Result<PointRevealPlan<VisualFieldViewGeneration>, PointRevealPlanError>>>>;

pub(crate) struct PendingPointRevealJob {
    pub(crate) handle: JobHandle,
    pub(crate) result: PointRevealJobResult,
    pub(crate) identity: PointRevealJobIdentity,
}

#[derive(Default)]
pub(crate) struct ScatterPointRevealState {
    pub(crate) config: PointRevealConfig,
    pub(crate) renderer: Option<ScatterPointRenderer>,
    pub(crate) stats: Option<PointRevealStats>,
    all_rows_mask: Option<rawscope_data::FilterMask>,
    dirty: bool,
}

impl WorkbenchApp {
    pub(crate) fn initialize_scatter_point_reveal(&mut self, gpu: &GpuContext) {
        self.point_reveal = ScatterPointRevealState {
            renderer: Some(ScatterPointRenderer::new(
                gpu.device(),
                gpu.surface_format(),
            )),
            all_rows_mask: Some(rawscope_data::FilterMask::all_included(
                self.scatter.points.len(),
            )),
            dirty: true,
            ..Default::default()
        };
        self.scatter.point_reveal_plan = None;
        self.scatter.point_reveal_frame = SemanticZoomFrame {
            density_alpha: 1.0,
            point_alpha: 0.0,
        };
    }

    pub(crate) fn invalidate_scatter_point_reveal(&mut self) {
        self.point_reveal.dirty = true;
        if let Some(pending) = self.scatter.pending_point_reveal_job.take() {
            pending.handle.cancel();
        }
        // Keep the last resident plan while a new settled plan is pending. The
        // render frame lowers point alpha during interaction, so this path is
        // still density-first without a pointer-time row scan.
        self.scatter.point_reveal_frame.point_alpha = 0.0;
    }

    pub(crate) fn reset_scatter_point_reveal_mask(&mut self) {
        self.point_reveal.all_rows_mask = Some(rawscope_data::FilterMask::all_included(
            self.scatter.points.len(),
        ));
        self.invalidate_scatter_point_reveal();
    }

    pub(crate) fn set_point_reveal_mode(&mut self, mode: PointRevealMode) {
        if self.point_reveal.config.mode == mode {
            return;
        }
        self.point_reveal.config.mode = mode;
        self.invalidate_scatter_point_reveal();
        if mode == PointRevealMode::Off {
            self.point_reveal.stats = None;
        }
        self.request_redraw();
    }

    /// Resolves one completed native point-plan job.  Completion is accepted
    /// only when every generation identity still matches the settled request.
    pub(crate) fn complete_point_reveal_job(
        &mut self,
        job_id: crate::job_coordinator::WorkbenchJobId,
    ) {
        let Some(pending) = self.scatter.pending_point_reveal_job.take() else {
            return;
        };
        if pending.handle.id() != job_id {
            self.scatter.pending_point_reveal_job = Some(pending);
            return;
        }
        let result = pending
            .result
            .lock()
            .expect("point reveal result lock")
            .take();
        let Some(result) = result else {
            warn!("point reveal job completed without a durable result");
            return;
        };
        if pending.handle.outcome() != Some(JobOutcome::Succeeded) {
            return;
        }
        let Ok(plan) = result else {
            self.point_reveal.dirty = true;
            return;
        };
        if !self.point_reveal_job_is_current(pending.identity) {
            return;
        }
        let Some(viewport) = self.scatter.viewport else {
            return;
        };
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        let Some(renderer) = self.point_reveal.renderer.as_mut() else {
            return;
        };
        if let Err(error) = renderer.update_plan(
            gpu.device(),
            gpu.queue(),
            &self.scatter.points,
            &plan,
            viewport.x_range(),
            viewport.y_range(),
            self.point_reveal.config,
        ) {
            error!(error = %error, "settled point reveal plan did not match resident points");
            self.point_reveal.dirty = true;
            return;
        }
        let plot_pixels = u64::from(self.scatter_plot_size().0)
            .saturating_mul(u64::from(self.scatter_plot_size().1));
        self.scatter.point_reveal_frame = semantic_zoom_frame(
            plan.eligible_count() as f32 / plot_pixels.max(1) as f32,
            self.semantic_zoom_policy(),
        );
        self.scatter.point_reveal_plan = Some(Arc::new(plan.clone()));
        self.point_reveal.stats = Some(PointRevealStats {
            eligible_count: plan.eligible_count() as usize,
            rendered_count: plan.rendered_count() as usize,
            sampled: plan.sampled(),
            blend: self.scatter.point_reveal_frame.point_alpha,
        });
        self.point_reveal.dirty = false;
        self.request_redraw();
    }

    pub(crate) fn prepare_scatter_point_reveal(&mut self) {
        let Some(plot) = self.plot_surface else {
            return;
        };
        let Some(viewport) = self.scatter.viewport else {
            return;
        };
        if self.render_schedule.is_refining() {
            self.scatter.point_reveal_frame.point_alpha = 0.0;
            return;
        }
        let plot_size = (plot.physical_rect.width, plot.physical_rect.height);
        let Some(cohort) = self.scatter_filters.cohort_snapshot.clone() else {
            return;
        };
        if self.point_reveal.config.mode == PointRevealMode::Off {
            self.scatter.point_reveal_frame = SemanticZoomFrame {
                density_alpha: 1.0,
                point_alpha: 0.0,
            };
            return;
        }
        if let Some(plan) = self.scatter.point_reveal_plan.as_ref() {
            let pixels = u64::from(plot_size.0).saturating_mul(u64::from(plot_size.1));
            self.scatter.point_reveal_frame = semantic_zoom_frame(
                plan.eligible_count() as f32 / pixels.max(1) as f32,
                self.semantic_zoom_policy(),
            );
        }
        if !self.point_reveal.dirty || self.scatter.pending_point_reveal_job.is_some() {
            return;
        }
        let Some(coordinator) = self.startup_coordinator.as_ref() else {
            return;
        };
        let Some(gpu_renderer) = self.scatter.density_renderer.as_ref() else {
            return;
        };
        let identity = PointRevealJobIdentity {
            dataset_generation: self.scatter_filters.dataset_generation,
            cohort_generation: cohort.cohort_generation(),
            view_generation: gpu_renderer.view_generation(),
            viewport_revision: self.render_schedule.settled_revision(),
        };
        let points = self
            .scatter
            .points
            .iter()
            .map(
                |point| rawscope_analysis::visual_field::ProjectedVisualPoint {
                    row_id: point.row_id,
                    x: f64::from(point.x),
                    y: f64::from(point.y),
                },
            )
            .collect::<Vec<_>>();
        let Some(x_domain) = rawscope_analysis::inspection::F64Domain::try_new(
            f64::from(viewport.x_range().min),
            f64::from(viewport.x_range().max),
        )
        .ok() else {
            return;
        };
        let Some(y_domain) = rawscope_analysis::inspection::F64Domain::try_new(
            f64::from(viewport.y_range().min),
            f64::from(viewport.y_range().max),
        )
        .ok() else {
            return;
        };
        let point_viewport = PointRevealViewport::new(x_domain, y_domain);
        let policy = self.semantic_zoom_policy();
        let result = Arc::new(Mutex::new(None));
        let result_for_job = Arc::clone(&result);
        let dataset_generation = identity.dataset_generation;
        let view_generation = identity.view_generation;
        let handle = match coordinator.submit_with_metadata(
            WorkbenchJobKind::PointReveal,
            WorkbenchJobGeneration(identity.viewport_revision),
            move |cancel: CancellationToken| {
                if cancel.is_cancelled() {
                    return JobOutcome::Cancelled;
                }
                let plan = build_point_reveal_plan_from_points(
                    &points,
                    dataset_generation,
                    &cohort,
                    point_viewport,
                    view_generation,
                    policy,
                );
                *result_for_job.lock().expect("point reveal result lock") = Some(plan);
                JobOutcome::Succeeded
            },
        ) {
            Ok(handle) => handle,
            Err(_) => return,
        };
        self.scatter.pending_point_reveal_job = Some(PendingPointRevealJob {
            handle,
            result,
            identity,
        });
    }

    pub(crate) fn refresh_point_reveal_emphasis(&mut self) {
        let emphasized = self
            .scatter_inspection
            .hovered
            .as_ref()
            .and_then(first_row_id)
            .or_else(|| {
                self.scatter_inspection
                    .pinned
                    .as_ref()
                    .and_then(|pinned| first_row_id(&pinned.summary.hit))
            });
        if let Some(renderer) = self.point_reveal.renderer.as_mut() {
            renderer.set_emphasized_row(emphasized);
        }
    }

    pub(crate) fn reproject_point_reveal_viewport(&mut self) {
        if let (Some(renderer), Some(viewport)) =
            (self.point_reveal.renderer.as_mut(), self.scatter.viewport)
        {
            renderer.set_viewport(viewport.x_range(), viewport.y_range());
        }
    }

    fn point_reveal_job_is_current(&self, identity: PointRevealJobIdentity) -> bool {
        let current = self
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .map(|snapshot| snapshot.cohort_generation());
        let current_identity = PointRevealJobIdentity {
            dataset_generation: self.scatter_filters.dataset_generation,
            cohort_generation: match current {
                Some(cohort_generation) => cohort_generation,
                None => return false,
            },
            view_generation: match self.scatter.density_renderer.as_ref() {
                Some(renderer) => renderer.view_generation(),
                None => return false,
            },
            viewport_revision: self.render_schedule.settled_revision(),
        };
        point_reveal_identity_matches(identity, current_identity)
    }

    fn semantic_zoom_policy(&self) -> SemanticZoomPolicy {
        SemanticZoomPolicy::try_new(
            self.point_reveal.config.fully_visible_rows_per_pixel,
            self.point_reveal.config.hidden_rows_per_pixel,
            self.point_reveal
                .config
                .max_rendered_points
                .min(u32::MAX as usize) as u32,
            self.point_reveal.config.radius_px,
        )
        .unwrap_or_else(|_| SemanticZoomPolicy::default())
    }

    fn scatter_plot_size(&self) -> (u32, u32) {
        self.plot_surface
            .map(|surface| (surface.physical_rect.width, surface.physical_rect.height))
            .unwrap_or((1, 1))
    }
}

fn first_row_id(hit: &rawscope_render::ScatterInspectionHit) -> Option<RowId> {
    hit.row_ids.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(viewport_revision: u64) -> PointRevealJobIdentity {
        let mut datasets = rawscope_data::DatasetGenerationCounter::default();
        let mut cohorts = rawscope_analysis::cohort::CohortGenerationCounter::default();
        let mut views = rawscope_render::VisualFieldViewGenerationCounter::default();
        PointRevealJobIdentity {
            dataset_generation: datasets.mint(),
            cohort_generation: cohorts.mint(),
            view_generation: views.mint(),
            viewport_revision,
        }
    }

    #[test]
    fn stale_point_plan_completion_is_rejected() {
        assert!(!point_reveal_identity_matches(identity(1), identity(2)));
    }

    #[test]
    fn hover_emphasis_does_not_rebuild_plan() {
        let mut app = WorkbenchApp::default();
        app.refresh_point_reveal_emphasis();
        assert!(app.scatter.point_reveal_plan.is_none());
        assert!(app.scatter.pending_point_reveal_job.is_none());
    }
}
