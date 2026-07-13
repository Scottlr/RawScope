//! Workbench coordination for resident normalized-difference density fields.

use std::error::Error;

use rawscope_data::ScatterPointRecord;
use rawscope_gpu::GpuContext;
use rawscope_render::{
    ComparisonFieldRenderer, ComparisonPresentation, ComparisonPresentationError, ComparisonSplit,
    DensityPresentationConfig,
};

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn initialize_scatter_difference(
        &mut self,
        gpu: &GpuContext,
        points: &[ScatterPointRecord],
        config: DensityPresentationConfig,
    ) -> Result<(), Box<dyn Error>> {
        let renderer = ComparisonFieldRenderer::new_with_palette(
            gpu.device(),
            gpu.queue(),
            gpu.surface_format(),
            points,
            config,
            self.palette_gpu_resources(gpu),
        )?;
        self.scatter.difference_stats = Some(renderer.stats());
        self.scatter.difference_renderer = Some(renderer);
        self.scatter.difference_baseline_dirty = false;
        Ok(())
    }

    pub(crate) fn replace_scatter_difference_dataset(&mut self) -> Result<(), Box<dyn Error>> {
        let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.difference_renderer.as_mut())
        else {
            return Ok(());
        };
        renderer.replace_dataset(
            gpu.device(),
            gpu.queue(),
            &self.scatter.points,
            self.scatter.density_dataset_revision,
        )?;
        self.scatter.difference_baseline_dirty = true;
        Ok(())
    }

    pub(crate) fn refresh_scatter_difference_density(
        &mut self,
        config: DensityPresentationConfig,
        baseline_dirty: bool,
    ) -> Result<(), Box<dyn Error>> {
        let (Some(gpu), Some(renderer)) =
            (self.gpu.as_ref(), self.scatter.difference_renderer.as_mut())
        else {
            return Ok(());
        };
        let active_total = self
            .scatter_filters
            .evaluation
            .as_ref()
            .map_or(self.scatter.points.len() as u64, |evaluation| {
                evaluation.included_count as u64
            });
        self.scatter.difference_stats = Some(renderer.update_fields(
            gpu.device(),
            gpu.queue(),
            config,
            baseline_dirty,
            active_total,
            self.render_schedule.settled_revision(),
        )?);
        self.scatter.difference_baseline_dirty = false;
        Ok(())
    }

    /// Publishes a comparison presentation without rebuilding either cohort field.
    pub(crate) fn set_scatter_comparison_presentation(
        &mut self,
        presentation: ComparisonPresentation,
    ) {
        let Some(renderer) = self.scatter.difference_renderer.as_mut() else {
            return;
        };
        if renderer.presentation() == presentation {
            return;
        }
        renderer.set_presentation(presentation);
        self.request_redraw();
    }

    /// Moves the split lens by changing its bounded uniform state only.
    pub(crate) fn set_scatter_comparison_split(
        &mut self,
        fraction: f32,
    ) -> Result<bool, ComparisonPresentationError> {
        let split = ComparisonSplit::new(fraction)?;
        let Some(renderer) = self.scatter.difference_renderer.as_mut() else {
            return Ok(false);
        };
        let changed = renderer.set_split(split);
        if changed {
            self.request_redraw();
        }
        Ok(changed)
    }
}
