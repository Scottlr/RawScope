//! Shared visual-field generation and transition identity.

use std::sync::Arc;

use rawscope_analysis::cohort::CohortGeneration;
use rawscope_core::{Generation, GenerationCounter, GridSize};
use rawscope_evidence::ScatterDensityPresentation;

use super::dataset_resources::VisualFieldDatasetGpuResources;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualFieldViewGeneration(Generation<VisualFieldViewOwner>);

impl VisualFieldViewGeneration {
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Default)]
pub struct VisualFieldViewGenerationCounter(GenerationCounter<VisualFieldViewOwner>);

impl VisualFieldViewGenerationCounter {
    pub fn mint(&mut self) -> VisualFieldViewGeneration {
        VisualFieldViewGeneration(self.0.mint())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisualFieldViewOwner;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualFieldQuality {
    Preview,
    Exact,
}

/// Metadata for one immutable field view over a shared dataset allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualFieldGeneration {
    dataset: Arc<VisualFieldDatasetGpuResources>,
    cohort_generation: CohortGeneration,
    view_generation: VisualFieldViewGeneration,
    grid: GridSize,
    quality: VisualFieldQuality,
}

impl VisualFieldGeneration {
    pub fn new(
        dataset: Arc<VisualFieldDatasetGpuResources>,
        cohort_generation: CohortGeneration,
        view_generation: VisualFieldViewGeneration,
        grid: GridSize,
        quality: VisualFieldQuality,
    ) -> Self {
        Self {
            dataset,
            cohort_generation,
            view_generation,
            grid,
            quality,
        }
    }

    pub fn dataset(&self) -> Arc<VisualFieldDatasetGpuResources> {
        Arc::clone(&self.dataset)
    }

    pub fn resources(&self) -> &VisualFieldDatasetGpuResources {
        &self.dataset
    }

    pub const fn cohort_generation(&self) -> CohortGeneration {
        self.cohort_generation
    }

    pub const fn view_generation(&self) -> VisualFieldViewGeneration {
        self.view_generation
    }

    pub const fn grid(&self) -> GridSize {
        self.grid
    }

    pub fn grid_width(&self) -> u32 {
        self.grid.width()
    }

    pub fn grid_height(&self) -> u32 {
        self.grid.height()
    }

    pub const fn quality(&self) -> VisualFieldQuality {
        self.quality
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualFieldTransitionField {
    pub field: VisualFieldGeneration,
    pub presentation: ScatterDensityPresentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDecision {
    Crossfade,
    ImmediateSwap,
}

pub fn transition_decision(
    previous: Option<VisualFieldTransitionField>,
    current: VisualFieldTransitionField,
    reduced_motion: bool,
) -> TransitionDecision {
    if reduced_motion
        || previous
            .as_ref()
            .is_none_or(|previous| !compatible(previous, &current))
    {
        TransitionDecision::ImmediateSwap
    } else {
        TransitionDecision::Crossfade
    }
}

fn compatible(previous: &VisualFieldTransitionField, current: &VisualFieldTransitionField) -> bool {
    previous.field.grid() == current.field.grid()
        && previous.presentation == current.presentation
        && previous.field.quality() == current.field.quality()
        && previous.field.resources() == current.field.resources()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rawscope_analysis::visual_field::{VisualFieldMapping, VisualFieldProjection};
    use rawscope_core::ColumnId;
    use rawscope_data::{DatasetGenerationCounter, DatasetSchema, StoreColumnKind};
    use rawscope_gpu::DeviceGeneration;

    use super::*;

    fn mapping(x: u32, y: u32) -> VisualFieldMapping {
        let schema =
            DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)])
                .unwrap();
        VisualFieldMapping::try_new(
            &schema,
            VisualFieldProjection::NumericPair {
                x: ColumnId::new(x),
                y: ColumnId::new(y),
            },
            None,
        )
        .unwrap()
    }

    fn resources(
        mapping: rawscope_analysis::visual_field::VisualFieldMapping,
    ) -> Arc<VisualFieldDatasetGpuResources> {
        let mut datasets = DatasetGenerationCounter::default();
        Arc::new(VisualFieldDatasetGpuResources::new(
            datasets.mint(),
            DeviceGeneration(1),
            mapping,
            10,
            40,
        ))
    }

    fn generation(
        dataset: Arc<VisualFieldDatasetGpuResources>,
        view: VisualFieldViewGeneration,
    ) -> VisualFieldGeneration {
        let mut cohorts = rawscope_analysis::cohort::CohortGenerationCounter::default();
        VisualFieldGeneration::new(
            dataset,
            cohorts.mint(),
            view,
            GridSize::new(16, 8),
            VisualFieldQuality::Exact,
        )
    }

    #[test]
    fn different_views_share_one_dataset_resource_arc() {
        let dataset = resources(mapping(0, 1));
        let mut views = VisualFieldViewGenerationCounter::default();
        let first = generation(Arc::clone(&dataset), views.mint());
        let second = generation(Arc::clone(&dataset), views.mint());

        assert!(Arc::ptr_eq(&first.dataset(), &second.dataset()));
        assert_ne!(first.view_generation(), second.view_generation());
    }

    #[test]
    fn visual_field_generation_rejects_mismatched_mapping() {
        let first_dataset = resources(mapping(0, 1));
        let second_dataset = resources(mapping(1, 0));
        let mut views = VisualFieldViewGenerationCounter::default();
        let first = VisualFieldTransitionField {
            field: generation(first_dataset, views.mint()),
            presentation: ScatterDensityPresentation::ExactCells,
        };
        let current = VisualFieldTransitionField {
            field: generation(second_dataset, views.mint()),
            presentation: ScatterDensityPresentation::ExactCells,
        };

        assert_eq!(
            transition_decision(Some(first), current, false),
            TransitionDecision::ImmediateSwap
        );
    }
}
