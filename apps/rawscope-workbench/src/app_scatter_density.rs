//! Scatter density recomputation and marginal refresh coordination.

use std::{error::Error, sync::Arc};

use rawscope_data::{generate_synthetic_points, SyntheticPointConfig};
use rawscope_gpu::GpuContext;
use rawscope_render::{
    DensityPresentationConfig, DensityReadbackPolicy, MassContourUniforms,
    ResidentExactFieldUpdate, ScatterMarginalSummary, ScatterViewport, SettledDensityContext,
    SummaryBin, VisualFieldCountGrid, VisualFieldGpuError, VisualFieldQuality,
};
use tracing::error;

use crate::app::{default_scatter_grid, WorkbenchApp, DEMO_SEED};
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
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(());
        };
        let Some(viewport) = self.scatter.viewport else {
            return Ok(());
        };
        let Some(scatter_density_renderer) = self.scatter.density_renderer.as_mut() else {
            return Ok(());
        };
        let exact_grid = default_scatter_grid(gpu, VisualFieldQuality::Exact)?;

        let renderer_config = DensityPresentationConfig::new(
            viewport.x_range(),
            viewport.y_range(),
            exact_grid.width(),
            exact_grid.height(),
        )
        .with_encoding(self.scatter.density_encoding)
        .with_presentation(self.scatter.density_presentation)
        .with_relief(self.scatter.relief_config);
        self.scatter.pending_settled_context_readback = false;
        let stats = {
            scatter_density_renderer.cancel_full_readback();
            let stats = scatter_density_renderer.update_density(
                gpu.device(),
                gpu.queue(),
                ResidentExactFieldUpdate {
                    config: renderer_config,
                    readback: DensityReadbackPolicy::None,
                },
            )?;
            scatter_density_renderer.begin_full_readback(gpu.device(), gpu.queue())?;
            stats
        };
        self.scatter.pending_settled_context_readback = true;
        self.scatter.render_stats = Some(stats);
        self.refresh_scatter_difference_density(renderer_config, true)?;
        self.render_schedule.exact_field_settled();
        self.rebuild_scatter_inspection_cache();
        self.update_window_title();
        self.request_redraw();

        Ok(())
    }

    pub(crate) fn refresh_scatter_marginal_summary(&mut self) {
        self.scatter.marginal_summary = self
            .scatter
            .settled_density_context
            .as_ref()
            .map(|context| scatter_marginal_summary_from_context(context));
    }

    pub(crate) fn queue_scatter_context_readback(
        &mut self,
        gpu: &GpuContext,
    ) -> Result<(), VisualFieldGpuError> {
        let Some(renderer) = self.scatter.density_renderer.as_mut() else {
            return Ok(());
        };
        renderer.begin_full_readback(gpu.device(), gpu.queue())?;
        self.scatter.pending_settled_context_readback = true;
        Ok(())
    }

    pub(crate) fn publish_scatter_density_context(
        &mut self,
        count_grid: VisualFieldCountGrid,
    ) -> Result<(), Box<dyn Error>> {
        let generation = self
            .scatter
            .density_renderer
            .as_ref()
            .ok_or("scatter density renderer disappeared before readback publication")?
            .view_generation();
        let counts = Arc::new(count_grid.into_density_count_grid());
        let context = Arc::new(SettledDensityContext::try_new_with_default_fractions(
            generation, counts,
        )?);
        if let Some(gpu) = self.gpu.as_ref() {
            if let Some(renderer) = self.scatter.density_renderer.as_mut() {
                renderer.set_settled_mass_contours(
                    gpu.queue(),
                    MassContourUniforms::from_settled_context(&context),
                );
            }
        }
        self.scatter.marginal_summary = Some(scatter_marginal_summary_from_context(&context));
        self.scatter.settled_density_context = Some(Arc::clone(&context));
        self.scatter_inspection.settled_context = Some(context);
        Ok(())
    }
}

fn scatter_marginal_summary_from_context(
    context: &SettledDensityContext,
) -> ScatterMarginalSummary {
    ScatterMarginalSummary {
        x_bins: summary_bins(&context.marginals.x_counts),
        y_bins: summary_bins(&context.marginals.y_counts),
        max_x_count: saturating_u32(context.marginals.max_x_count),
        max_y_count: saturating_u32(context.marginals.max_y_count),
    }
}

fn summary_bins(counts: &[u64]) -> Vec<SummaryBin> {
    counts
        .iter()
        .copied()
        .enumerate()
        .map(|(index, count)| SummaryBin {
            index: index as u32,
            count: saturating_u32(count),
        })
        .collect()
}

fn saturating_u32(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}
