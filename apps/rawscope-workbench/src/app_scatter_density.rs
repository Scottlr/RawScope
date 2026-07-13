//! Scatter density recomputation and marginal refresh coordination.

use std::error::Error;

use rawscope_data::{generate_synthetic_points, SyntheticPointConfig};
use rawscope_render::{
    scatter_marginal_summary, scatter_marginal_summary_masked, DensityPresentationConfig,
    DensityReadbackPolicy, ResidentExactFieldUpdate, ScatterViewport,
};
use tracing::error;

use crate::app::{WorkbenchApp, DEMO_GRID_HEIGHT, DEMO_GRID_WIDTH, DEMO_SEED, MARGINAL_BIN_COUNT};
use crate::{
    app_scatter_filter::ScatterFilterState, app_scatter_inspection::ScatterInspectionState,
    app_scatter_projection::ScatterProjectionState, demo::PointCountPreset,
};

impl WorkbenchApp {
    pub(crate) fn switch_point_preset(&mut self, preset: PointCountPreset) {
        if !self.demo_mode.is_scatter() {
            return;
        }
        if self.input.is_some() {
            return;
        }

        let preset_is_already_active = self.scatter.active_preset == preset;
        if preset_is_already_active {
            return;
        }
        self.cancel_visual_transition();

        let dataset =
            generate_synthetic_points(SyntheticPointConfig::new(DEMO_SEED, preset.row_count));
        let viewport = ScatterViewport::new(dataset.x_range, dataset.y_range);
        self.scatter.points = dataset.points;
        self.scatter.source_rows = None;
        self.scatter_filters = ScatterFilterState::default();
        self.scatter_inspection = ScatterInspectionState::default();
        self.scatter_projection = ScatterProjectionState::default();
        self.reset_scatter_point_reveal_mask();
        self.clear_dataset_diff_state();
        self.scatter.active_preset = preset;
        self.scatter.point_count_label = preset.row_count_label().to_string();
        self.workbench_state.dataset_identity = Some(dataset.identity);
        self.workbench_state.active_dataset_profile = None;
        self.workbench_state.dataset_metadata = Some(dataset.metadata);
        self.clear_active_selection();
        self.scatter.viewport = Some(viewport);
        self.workbench_state.export_status = crate::ui::ExportStatus::Idle;
        self.clear_brush();
        self.rebuild_missingness_state();
        self.rebuild_scatter_aggregate_overview();

        self.scatter.density_dataset_revision += 1;
        if let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.density_renderer.as_mut())
        {
            if let Err(err) = renderer.replace_dataset(
                gpu.device(),
                gpu.queue(),
                &self.scatter.points,
                self.scatter.density_dataset_revision,
            ) {
                error!(error = %err, "failed to replace resident scatter dataset");
                return;
            }
        }
        if let Err(err) = self.replace_scatter_difference_dataset() {
            error!(error = %err, "failed to replace difference scatter dataset");
            return;
        }

        if let Err(err) = self.recompute_density() {
            error!(
                error = %err,
                point_count = preset.row_count,
                "failed to recompute scatter density after preset change"
            );
        }
    }

    pub(crate) fn recompute_density(&mut self) -> Result<(), Box<dyn Error>> {
        self.refresh_scatter_marginal_summary();
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.scatter.viewport else {
            return Ok(());
        };
        let Some(scatter_density_renderer) = self.scatter.density_renderer.as_mut() else {
            return Ok(());
        };

        let renderer_config = DensityPresentationConfig::new(
            viewport.x_range(),
            viewport.y_range(),
            DEMO_GRID_WIDTH,
            DEMO_GRID_HEIGHT,
        )
        .with_encoding(self.scatter.density_encoding)
        .with_presentation(self.scatter.density_presentation)
        .with_relief(self.scatter.relief_config);
        let stats = scatter_density_renderer.update_density(
            gpu.device(),
            gpu.queue(),
            ResidentExactFieldUpdate {
                config: renderer_config,
                readback: DensityReadbackPolicy::None,
            },
        )?;
        self.scatter.render_stats = Some(stats);
        self.refresh_scatter_difference_density(renderer_config, true)?;
        self.render_schedule.exact_field_settled();
        self.rebuild_scatter_inspection_cache();
        self.update_window_title();
        self.request_redraw();

        Ok(())
    }

    pub(crate) fn refresh_scatter_marginal_summary(&mut self) {
        let Some(viewport) = self.scatter.viewport else {
            self.scatter.marginal_summary = None;
            return;
        };

        if let Some(snapshot) = self.scatter_filters.cohort_snapshot.as_ref() {
            let mask = snapshot.filter_mask();
            self.scatter.marginal_summary = Some(
                scatter_marginal_summary_masked(
                    &self.scatter.points,
                    &mask,
                    viewport.x_range(),
                    viewport.y_range(),
                    MARGINAL_BIN_COUNT,
                    MARGINAL_BIN_COUNT,
                )
                .expect("cohort snapshot remains aligned with scatter points"),
            );
            return;
        }

        self.scatter.marginal_summary = self
            .scatter_filters
            .evaluation
            .as_ref()
            .map(|evaluation| {
                scatter_marginal_summary_masked(
                    &self.scatter.points,
                    &evaluation.mask,
                    viewport.x_range(),
                    viewport.y_range(),
                    MARGINAL_BIN_COUNT,
                    MARGINAL_BIN_COUNT,
                )
                .expect("filter evaluation remains aligned with scatter points")
            })
            .or_else(|| {
                Some(scatter_marginal_summary(
                    &self.scatter.points,
                    viewport.x_range(),
                    viewport.y_range(),
                    MARGINAL_BIN_COUNT,
                    MARGINAL_BIN_COUNT,
                ))
            });
    }
}
