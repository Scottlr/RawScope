//! Private scatter-domain command owner.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ScatterGeneration {
    pub(crate) dataset: u64,
    pub(crate) cohort: u64,
    pub(crate) viewport: u64,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScatterCommand {
    SetCohort {
        generation: ScatterGeneration,
    },
    SetViewport {
        generation: ScatterGeneration,
    },
    SetProjection {
        generation: ScatterGeneration,
        projection: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScatterResult {
    Prepared {
        generation: ScatterGeneration,
    },
    ProjectionPrepared {
        generation: ScatterGeneration,
        projection: u8,
    },
    RejectedStale {
        generation: ScatterGeneration,
    },
}

#[derive(Debug, Default)]
pub(crate) struct ScatterController {
    generation: Option<ScatterGeneration>,
    projection: u8,
}

impl ScatterController {
    pub(crate) fn handle(&mut self, command: ScatterCommand) -> ScatterResult {
        let generation = match command {
            ScatterCommand::SetCohort { generation }
            | ScatterCommand::SetViewport { generation }
            | ScatterCommand::SetProjection { generation, .. } => generation,
        };
        if self.generation.is_some_and(|current| generation < current) {
            return ScatterResult::RejectedStale { generation };
        }
        match command {
            ScatterCommand::SetProjection { projection, .. } => {
                self.projection = projection;
                self.generation = Some(generation);
                ScatterResult::ProjectionPrepared {
                    generation,
                    projection,
                }
            }
            ScatterCommand::SetCohort { .. } | ScatterCommand::SetViewport { .. } => {
                self.generation = Some(generation);
                ScatterResult::Prepared { generation }
            }
        }
    }

    pub(crate) fn generation(&self) -> Option<ScatterGeneration> {
        self.generation
    }
    pub(crate) fn projection(&self) -> u8 {
        self.projection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn generation(dataset: u64, viewport: u64) -> ScatterGeneration {
        ScatterGeneration {
            dataset,
            cohort: 2,
            viewport,
        }
    }
    #[test]
    fn scatter_commands_publish_generation_tagged_results() {
        let mut controller = ScatterController::default();
        let generation = generation(1, 4);
        assert_eq!(
            controller.handle(ScatterCommand::SetViewport { generation }),
            ScatterResult::Prepared { generation }
        );
        assert_eq!(
            controller.handle(ScatterCommand::SetCohort { generation }),
            ScatterResult::Prepared { generation }
        );
        assert_eq!(controller.generation(), Some(generation));
    }
    #[test]
    fn stale_dataset_commands_do_not_replace_current_state() {
        let mut controller = ScatterController::default();
        let current = generation(2, 1);
        controller.handle(ScatterCommand::SetViewport {
            generation: current,
        });
        let stale = generation(1, 9);
        assert_eq!(
            controller.handle(ScatterCommand::SetProjection {
                generation: stale,
                projection: 3
            }),
            ScatterResult::RejectedStale { generation: stale }
        );
        assert_eq!(controller.projection(), 0);
    }

    #[test]
    fn stale_cohort_commands_do_not_replace_current_state() {
        let mut controller = ScatterController::default();
        let current = ScatterGeneration {
            dataset: 2,
            cohort: 4,
            viewport: 1,
        };
        controller.handle(ScatterCommand::SetViewport {
            generation: current,
        });
        let stale = ScatterGeneration {
            dataset: 2,
            cohort: 3,
            viewport: 9,
        };
        assert_eq!(
            controller.handle(ScatterCommand::SetProjection {
                generation: stale,
                projection: 3,
            }),
            ScatterResult::RejectedStale { generation: stale }
        );
        assert_eq!(controller.projection(), 0);
    }
}
