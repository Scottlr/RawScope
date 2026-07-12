//! Private timeline-domain command owner.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TimelineGeneration {
    pub(crate) dataset: u64,
    pub(crate) cohort: u64,
    pub(crate) viewport: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimelineCommand {
    SetCohort { generation: TimelineGeneration },
    SetViewport { generation: TimelineGeneration },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimelineResult {
    Prepared { generation: TimelineGeneration },
    RejectedStale { generation: TimelineGeneration },
}

#[derive(Debug, Default)]
pub(crate) struct TimelineController {
    generation: Option<TimelineGeneration>,
}

impl TimelineController {
    pub(crate) fn handle(&mut self, command: TimelineCommand) -> TimelineResult {
        let generation = match command {
            TimelineCommand::SetCohort { generation }
            | TimelineCommand::SetViewport { generation } => generation,
        };
        if self.generation.is_some_and(|current| generation < current) {
            return TimelineResult::RejectedStale { generation };
        }
        self.generation = Some(generation);
        TimelineResult::Prepared { generation }
    }
    pub(crate) fn generation(&self) -> Option<TimelineGeneration> {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timeline_owner_rejects_old_dataset_generations() {
        let mut controller = TimelineController::default();
        let current = TimelineGeneration {
            dataset: 4,
            cohort: 1,
            viewport: 2,
        };
        controller.handle(TimelineCommand::SetViewport {
            generation: current,
        });
        let stale = TimelineGeneration {
            dataset: 3,
            cohort: 9,
            viewport: 9,
        };
        assert_eq!(
            controller.handle(TimelineCommand::SetCohort { generation: stale }),
            TimelineResult::RejectedStale { generation: stale }
        );
        assert_eq!(controller.generation(), Some(current));
    }

    #[test]
    fn timeline_owner_rejects_old_cohort_generations() {
        let mut controller = TimelineController::default();
        let current = TimelineGeneration {
            dataset: 4,
            cohort: 2,
            viewport: 2,
        };
        controller.handle(TimelineCommand::SetViewport {
            generation: current,
        });
        let stale = TimelineGeneration {
            dataset: 4,
            cohort: 1,
            viewport: 9,
        };
        assert_eq!(
            controller.handle(TimelineCommand::SetCohort { generation: stale }),
            TimelineResult::RejectedStale { generation: stale }
        );
        assert_eq!(controller.generation(), Some(current));
    }
}
