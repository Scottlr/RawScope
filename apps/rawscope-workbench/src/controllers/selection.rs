//! Private selection-domain command owner.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionGeneration {
    pub(crate) dataset: u64,
    pub(crate) cohort: u64,
    pub(crate) selection: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionCommand {
    pub(crate) generation: SelectionGeneration,
    pub(crate) selected_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionResult {
    Published {
        generation: SelectionGeneration,
        selected_count: u64,
    },
    RejectedStale {
        generation: SelectionGeneration,
    },
}

#[derive(Debug, Default)]
pub(crate) struct SelectionController {
    current: Option<SelectionCommand>,
}

impl SelectionController {
    pub(crate) fn publish(&mut self, command: SelectionCommand) -> SelectionResult {
        if self.current.is_some_and(|current| {
            (
                command.generation.dataset,
                command.generation.cohort,
                command.generation.selection,
            ) < (
                current.generation.dataset,
                current.generation.cohort,
                current.generation.selection,
            )
        }) {
            return SelectionResult::RejectedStale {
                generation: command.generation,
            };
        }
        self.current = Some(command);
        SelectionResult::Published {
            generation: command.generation,
            selected_count: command.selected_count,
        }
    }
    pub(crate) fn current(&self) -> Option<SelectionCommand> {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_owner_rejects_out_of_order_generations() {
        let mut controller = SelectionController::default();
        let current = SelectionGeneration {
            dataset: 1,
            cohort: 2,
            selection: 4,
        };
        controller.publish(SelectionCommand {
            generation: current,
            selected_count: 3,
        });
        let stale = SelectionGeneration {
            dataset: 1,
            cohort: 2,
            selection: 3,
        };
        assert_eq!(
            controller.publish(SelectionCommand {
                generation: stale,
                selected_count: 9
            }),
            SelectionResult::RejectedStale { generation: stale }
        );
        assert_eq!(controller.current().unwrap().selected_count, 3);
    }
}
