//! Compatibility rules for retaining a completed scatter field during a transition.

use crate::{ScatterDatasetGpuResources, ScatterDensityPresentation, ScatterFieldGeneration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterTransitionField {
    pub field: ScatterFieldGeneration,
    pub grid_width: u32,
    pub grid_height: u32,
    pub presentation: ScatterDensityPresentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDecision {
    Crossfade,
    ImmediateSwap,
}

pub fn transition_decision(
    previous: Option<ScatterTransitionField>,
    current: ScatterTransitionField,
    reduced_motion: bool,
) -> TransitionDecision {
    if reduced_motion || previous.is_none_or(|previous| !compatible(previous, current)) {
        TransitionDecision::ImmediateSwap
    } else {
        TransitionDecision::Crossfade
    }
}

fn compatible(previous: ScatterTransitionField, current: ScatterTransitionField) -> bool {
    previous.grid_width == current.grid_width
        && previous.grid_height == current.grid_height
        && previous.presentation == current.presentation
        && same_resource_generation(previous.field.resources(), current.field.resources())
}

fn same_resource_generation(
    previous: ScatterDatasetGpuResources,
    current: ScatterDatasetGpuResources,
) -> bool {
    previous.dataset_generation() == current.dataset_generation()
        && previous.device_generation() == current.device_generation()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::cohort::CohortGenerationCounter;
    use rawscope_data::DatasetGenerationCounter;
    use rawscope_gpu::DeviceGeneration;

    fn field(device: DeviceGeneration, width: u32) -> ScatterTransitionField {
        let mut datasets = DatasetGenerationCounter::default();
        let mut cohorts = CohortGenerationCounter::default();
        let mut views = crate::ScatterViewGenerationCounter::default();
        let resources = ScatterDatasetGpuResources::new(datasets.mint(), device, 10, 40);
        ScatterTransitionField {
            field: ScatterFieldGeneration::new(resources, cohorts.mint(), views.mint()),
            grid_width: width,
            grid_height: 8,
            presentation: ScatterDensityPresentation::ExactCells,
        }
    }

    #[test]
    fn incompatible_grid_or_device_swaps_immediately() {
        let current = field(DeviceGeneration(1), 16);
        assert_eq!(
            transition_decision(Some(field(DeviceGeneration(1), 32)), current, false),
            TransitionDecision::ImmediateSwap
        );
        assert_eq!(
            transition_decision(Some(field(DeviceGeneration(2), 16)), current, false),
            TransitionDecision::ImmediateSwap
        );
    }

    #[test]
    fn reduced_motion_never_retains_previous_field() {
        let current = field(DeviceGeneration(1), 16);
        assert_eq!(
            transition_decision(Some(current), current, true),
            TransitionDecision::ImmediateSwap
        );
    }
}
