//! Settled point-reveal selection and GPU overlay coordination.

use rawscope_core::RowId;
use rawscope_data::{FilterMask, FilterRevision};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    select_points_for_reveal, PointRevealConfig, PointRevealMode, PointRevealStats,
    ScatterPointRenderer,
};
use tracing::error;

use crate::app::WorkbenchApp;

#[derive(Default)]
pub(crate) struct ScatterPointRevealState {
    pub(crate) config: PointRevealConfig,
    pub(crate) renderer: Option<ScatterPointRenderer>,
    pub(crate) stats: Option<PointRevealStats>,
    all_rows_mask: Option<FilterMask>,
    cache_viewport_revision: u64,
    cache_filter_revision: FilterRevision,
    cache_plot_size: (u32, u32),
    dirty: bool,
}

impl WorkbenchApp {
    pub(crate) fn initialize_scatter_point_reveal(&mut self, gpu: &GpuContext) {
        self.point_reveal = ScatterPointRevealState {
            renderer: Some(ScatterPointRenderer::new(
                gpu.device(),
                gpu.surface_format(),
            )),
            all_rows_mask: Some(FilterMask::all_included(self.scatter.points.len())),
            dirty: true,
            ..Default::default()
        };
    }

    pub(crate) fn invalidate_scatter_point_reveal(&mut self) {
        self.point_reveal.dirty = true;
        self.point_reveal.stats = None;
        if let Some(renderer) = self.point_reveal.renderer.as_mut() {
            renderer.hide();
        }
    }

    pub(crate) fn reset_scatter_point_reveal_mask(&mut self) {
        self.point_reveal.all_rows_mask = Some(FilterMask::all_included(self.scatter.points.len()));
        self.point_reveal.dirty = true;
    }

    pub(crate) fn set_point_reveal_mode(&mut self, mode: PointRevealMode) {
        if self.point_reveal.config.mode == mode {
            return;
        }
        self.point_reveal.config.mode = mode;
        self.point_reveal.dirty = true;
        self.request_redraw();
    }

    pub(crate) fn prepare_scatter_point_reveal(&mut self) {
        if self.render_schedule.is_refining() {
            return;
        }
        let (Some(plot), Some(viewport), Some(gpu)) =
            (self.plot_surface, self.scatter.viewport, self.gpu.as_ref())
        else {
            return;
        };
        let plot_size = (plot.physical_rect.width, plot.physical_rect.height);
        let filter_revision = self
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .map(|snapshot| snapshot.filter_revision())
            .or_else(|| {
                self.scatter_filters
                    .evaluation
                    .as_ref()
                    .map(|evaluation| evaluation.revision)
            })
            .unwrap_or_default();
        let viewport_revision = self.render_schedule.settled_revision();
        let cache_is_current = !self.point_reveal.dirty
            && self.point_reveal.cache_viewport_revision == viewport_revision
            && self.point_reveal.cache_filter_revision == filter_revision
            && self.point_reveal.cache_plot_size == plot_size;
        if cache_is_current {
            return;
        }
        let cohort_mask = self
            .scatter_filters
            .cohort_snapshot
            .as_ref()
            .map(|snapshot| snapshot.filter_mask());
        let mask = cohort_mask
            .as_ref()
            .or_else(|| {
                self.scatter_filters
                    .evaluation
                    .as_ref()
                    .map(|evaluation| &evaluation.mask)
            })
            .or(self.point_reveal.all_rows_mask.as_ref());
        let Some(mask) = mask else { return };
        let selection = match select_points_for_reveal(
            &self.scatter.points,
            mask,
            viewport.x_range(),
            viewport.y_range(),
            plot_size.0,
            plot_size.1,
            self.point_reveal.config,
        ) {
            Ok(selection) => selection,
            Err(err) => {
                error!(error = %err, "failed to select settled scatter point reveal");
                return;
            }
        };
        let Some(renderer) = self.point_reveal.renderer.as_mut() else {
            return;
        };
        renderer.update_selection(
            gpu.device(),
            gpu.queue(),
            &self.scatter.points,
            &selection,
            viewport.x_range(),
            viewport.y_range(),
            self.point_reveal.config,
        );
        self.point_reveal.stats = Some(selection.stats());
        self.point_reveal.cache_viewport_revision = viewport_revision;
        self.point_reveal.cache_filter_revision = filter_revision;
        self.point_reveal.cache_plot_size = plot_size;
        self.point_reveal.dirty = false;
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
}

fn first_row_id(hit: &rawscope_render::ScatterInspectionHit) -> Option<RowId> {
    hit.row_ids.first().copied()
}
