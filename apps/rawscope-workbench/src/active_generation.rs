//! Immutable active-workbench generations and their atomic publication boundary.

use std::{fmt, sync::Arc};

use rawscope_analysis::{
    cohort::CohortSnapshot, projection::ProjectedScatterGeneration, selection::SelectionSnapshot,
};
use rawscope_data::DatasetGeneration;

use crate::render_coordinator::SettledRenderGeneration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WorkbenchGenerationId(u64);

impl WorkbenchGenerationId {
    pub(crate) const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkbenchProjection {
    dataset: DatasetGeneration,
    scatter: Option<Arc<ProjectedScatterGeneration>>,
}

impl WorkbenchProjection {
    pub(crate) fn scatter(
        dataset: DatasetGeneration,
        projection: Arc<ProjectedScatterGeneration>,
    ) -> Self {
        Self {
            dataset,
            scatter: Some(projection),
        }
    }

    pub(crate) fn timeline(dataset: DatasetGeneration) -> Self {
        Self {
            dataset,
            scatter: None,
        }
    }

    fn dataset(&self) -> DatasetGeneration {
        self.dataset
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActiveWorkbenchGeneration {
    id: WorkbenchGenerationId,
    dataset: DatasetGeneration,
    cohort: Arc<CohortSnapshot>,
    projection: WorkbenchProjection,
    render: SettledRenderGeneration,
    selection: Arc<SelectionSnapshot>,
}

impl ActiveWorkbenchGeneration {
    pub(crate) fn id(&self) -> WorkbenchGenerationId {
        self.id
    }

    pub(crate) fn dataset(&self) -> DatasetGeneration {
        self.dataset
    }

    pub(crate) fn cohort(&self) -> &CohortSnapshot {
        &self.cohort
    }

    pub(crate) fn projection(&self) -> &WorkbenchProjection {
        &self.projection
    }

    pub(crate) fn render(&self) -> SettledRenderGeneration {
        self.render
    }

    pub(crate) fn selection(&self) -> &SelectionSnapshot {
        &self.selection
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActiveWorkbenchCandidate {
    pub(crate) dataset: DatasetGeneration,
    pub(crate) cohort: Arc<CohortSnapshot>,
    pub(crate) projection: WorkbenchProjection,
    pub(crate) render: SettledRenderGeneration,
    pub(crate) selection: Arc<SelectionSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveGenerationError {
    DatasetMismatch {
        expected: DatasetGeneration,
        actual: DatasetGeneration,
    },
    CohortDatasetMismatch {
        expected: DatasetGeneration,
        actual: DatasetGeneration,
    },
    ProjectionDatasetMismatch {
        expected: DatasetGeneration,
        actual: DatasetGeneration,
    },
    RenderDatasetMismatch {
        expected: DatasetGeneration,
        actual: u64,
    },
    RenderCohortMismatch {
        expected: u64,
        actual: u64,
    },
    SelectionDatasetMismatch {
        expected: DatasetGeneration,
        actual: DatasetGeneration,
    },
    SelectionCohortMismatch {
        expected: u64,
        actual: u64,
    },
    IdExhausted,
}

impl fmt::Display for ActiveGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatasetMismatch { .. } => {
                formatter.write_str("active generation dataset mismatch")
            }
            Self::CohortDatasetMismatch { .. } => {
                formatter.write_str("active generation cohort dataset mismatch")
            }
            Self::ProjectionDatasetMismatch { .. } => {
                formatter.write_str("active generation projection dataset mismatch")
            }
            Self::RenderDatasetMismatch { .. } => {
                formatter.write_str("active generation render dataset mismatch")
            }
            Self::RenderCohortMismatch { .. } => {
                formatter.write_str("active generation render cohort mismatch")
            }
            Self::SelectionDatasetMismatch { .. } => {
                formatter.write_str("active generation selection dataset mismatch")
            }
            Self::SelectionCohortMismatch { .. } => {
                formatter.write_str("active generation selection cohort mismatch")
            }
            Self::IdExhausted => formatter.write_str("workbench generation id exhausted"),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ActiveWorkbenchState {
    next_id: u64,
    active: Option<Arc<ActiveWorkbenchGeneration>>,
}

impl ActiveWorkbenchState {
    pub(crate) fn active(&self) -> Option<&Arc<ActiveWorkbenchGeneration>> {
        self.active.as_ref()
    }

    pub(crate) fn commit(
        &mut self,
        candidate: ActiveWorkbenchCandidate,
    ) -> Result<Arc<ActiveWorkbenchGeneration>, ActiveGenerationError> {
        let generation = self.validate(candidate)?;
        let active = Arc::new(generation);
        self.active = Some(Arc::clone(&active));
        Ok(active)
    }

    fn validate(
        &mut self,
        candidate: ActiveWorkbenchCandidate,
    ) -> Result<ActiveWorkbenchGeneration, ActiveGenerationError> {
        let dataset = candidate.dataset;
        let cohort = candidate.cohort;
        let projection = candidate.projection;
        let render = candidate.render;
        let selection = candidate.selection;

        if cohort.dataset_generation() != dataset {
            return Err(ActiveGenerationError::CohortDatasetMismatch {
                expected: dataset,
                actual: cohort.dataset_generation(),
            });
        }
        if projection.dataset() != dataset {
            return Err(ActiveGenerationError::ProjectionDatasetMismatch {
                expected: dataset,
                actual: projection.dataset(),
            });
        }
        if render.viewport.dataset_generation != dataset.get() {
            return Err(ActiveGenerationError::RenderDatasetMismatch {
                expected: dataset,
                actual: render.viewport.dataset_generation,
            });
        }
        let cohort_generation = cohort.cohort_generation().get();
        if render.viewport.cohort_generation != cohort_generation {
            return Err(ActiveGenerationError::RenderCohortMismatch {
                expected: cohort_generation,
                actual: render.viewport.cohort_generation,
            });
        }
        if selection.dataset_generation() != dataset {
            return Err(ActiveGenerationError::SelectionDatasetMismatch {
                expected: dataset,
                actual: selection.dataset_generation(),
            });
        }
        if selection.cohort_generation().get() != cohort_generation {
            return Err(ActiveGenerationError::SelectionCohortMismatch {
                expected: cohort_generation,
                actual: selection.cohort_generation().get(),
            });
        }

        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ActiveGenerationError::IdExhausted)?;
        Ok(ActiveWorkbenchGeneration {
            id: WorkbenchGenerationId(self.next_id),
            dataset,
            cohort,
            projection,
            render,
            selection,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::{
        cohort::{CohortBuilder, CohortGenerationCounter, CohortPolicy},
        selection::SelectionSnapshot,
    };
    use rawscope_core::{RowId, SelectionId};
    use rawscope_data::{
        build_visual_field_catalog, DatasetGenerationCounter, LoadedSourceRow, LoadedSourceTable,
        VisualFieldCatalogConfig,
    };

    fn candidate() -> ActiveWorkbenchCandidate {
        let mut dataset_generations = DatasetGenerationCounter::default();
        let dataset = dataset_generations.mint();
        let source = LoadedSourceTable {
            columns: vec![],
            rows: vec![LoadedSourceRow {
                row_id: RowId(0),
                values: vec![],
            }],
        };
        let catalog = build_visual_field_catalog(&source, VisualFieldCatalogConfig::default());
        let mut cohort_builder = CohortBuilder::new(CohortPolicy::default());
        let mut cohort_generations = CohortGenerationCounter::default();
        let cohort = Arc::new(
            cohort_builder
                .evaluate(&source, &catalog, dataset, &mut cohort_generations)
                .unwrap(),
        );
        let cohort_generation = cohort.cohort_generation();
        let selection = Arc::new(
            SelectionSnapshot::from_parts(
                dataset,
                cohort_generation,
                SelectionId(1),
                [RowId(0)],
                [0],
                1,
            )
            .unwrap(),
        );
        let render = SettledRenderGeneration {
            viewport: crate::render_coordinator::ViewportIntent {
                dataset_generation: dataset.get(),
                cohort_generation: cohort_generation.get(),
                viewport_generation: 1,
            },
            request_generation: 1,
        };
        let projection = WorkbenchProjection::timeline(dataset);
        ActiveWorkbenchCandidate {
            dataset,
            cohort,
            projection,
            render,
            selection,
        }
    }

    #[test]
    fn matching_candidate_publishes_one_arc_generation() {
        let mut state = ActiveWorkbenchState::default();
        let active = state.commit(candidate()).unwrap();
        assert_eq!(active.id().get(), 1);
        assert!(Arc::ptr_eq(state.active().unwrap(), &active));
    }

    #[test]
    fn mismatched_selection_keeps_previous_active_arc() {
        let mut state = ActiveWorkbenchState::default();
        let first = state.commit(candidate()).unwrap();
        let mut stale = candidate();
        let mut cohort_generations = CohortGenerationCounter::default();
        let _ = cohort_generations.mint();
        let stale_cohort = cohort_generations.mint();
        stale.selection = Arc::new(
            SelectionSnapshot::from_parts(
                stale.dataset,
                stale_cohort,
                SelectionId(2),
                [RowId(0)],
                [0],
                1,
            )
            .unwrap(),
        );
        assert!(matches!(
            state.commit(stale),
            Err(ActiveGenerationError::SelectionCohortMismatch { .. })
        ));
        assert!(Arc::ptr_eq(state.active().unwrap(), &first));
    }
}
