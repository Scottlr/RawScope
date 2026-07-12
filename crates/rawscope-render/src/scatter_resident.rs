//! Generation-safe ownership contracts for shared scatter GPU resources.

use rawscope_analysis::cohort::CohortGeneration;
use rawscope_core::{Generation, GenerationCounter};
use rawscope_data::DatasetGeneration;
use rawscope_gpu::DeviceGeneration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScatterViewGeneration(Generation<ScatterViewOwner>);

impl ScatterViewGeneration {
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Default)]
pub struct ScatterViewGenerationCounter(GenerationCounter<ScatterViewOwner>);

impl ScatterViewGenerationCounter {
    pub fn mint(&mut self) -> ScatterViewGeneration {
        ScatterViewGeneration(self.0.mint())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ScatterViewOwner;

/// Identity and accounting for one immutable packed point-buffer generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterDatasetGpuResources {
    dataset_generation: DatasetGeneration,
    device_generation: DeviceGeneration,
    point_count: u64,
    allocated_bytes: u64,
}

impl ScatterDatasetGpuResources {
    pub fn new(
        dataset_generation: DatasetGeneration,
        device_generation: DeviceGeneration,
        point_count: u64,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            dataset_generation,
            device_generation,
            point_count,
            allocated_bytes,
        }
    }

    pub const fn dataset_generation(self) -> DatasetGeneration {
        self.dataset_generation
    }
    pub const fn device_generation(self) -> DeviceGeneration {
        self.device_generation
    }
    pub const fn point_count(self) -> u64 {
        self.point_count
    }
    pub const fn allocated_bytes(self) -> u64 {
        self.allocated_bytes
    }
}

/// View metadata that may reference the shared dataset allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterFieldGeneration {
    resources: ScatterDatasetGpuResources,
    cohort_generation: CohortGeneration,
    view_generation: ScatterViewGeneration,
}

impl ScatterFieldGeneration {
    pub fn new(
        resources: ScatterDatasetGpuResources,
        cohort_generation: CohortGeneration,
        view_generation: ScatterViewGeneration,
    ) -> Self {
        Self {
            resources,
            cohort_generation,
            view_generation,
        }
    }

    pub const fn resources(self) -> ScatterDatasetGpuResources {
        self.resources
    }
    pub const fn cohort_generation(self) -> CohortGeneration {
        self.cohort_generation
    }
    pub const fn view_generation(self) -> ScatterViewGeneration {
        self.view_generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::cohort::CohortGenerationCounter;
    use rawscope_data::DatasetGenerationCounter;

    #[test]
    fn field_generation_references_one_dataset_resource_identity() {
        let mut datasets = DatasetGenerationCounter::default();
        let mut cohorts = CohortGenerationCounter::default();
        let mut views = ScatterViewGenerationCounter::default();
        let resources = ScatterDatasetGpuResources::new(
            datasets.mint(),
            DeviceGeneration(7),
            2_000_000,
            16_000_000,
        );
        let field = ScatterFieldGeneration::new(resources, cohorts.mint(), views.mint());
        assert_eq!(field.resources(), resources);
        assert_eq!(field.resources().point_count(), 2_000_000);
    }

    #[test]
    fn different_views_can_share_the_same_immutable_resources() {
        let mut datasets = DatasetGenerationCounter::default();
        let mut cohorts = CohortGenerationCounter::default();
        let mut views = ScatterViewGenerationCounter::default();
        let resources =
            ScatterDatasetGpuResources::new(datasets.mint(), DeviceGeneration(1), 4, 128);
        let first = ScatterFieldGeneration::new(resources, cohorts.mint(), views.mint());
        let second = ScatterFieldGeneration::new(resources, cohorts.mint(), views.mint());
        assert_eq!(first.resources(), second.resources());
        assert_ne!(first.view_generation(), second.view_generation());
    }
}
