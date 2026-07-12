//! Atomic RAM/VRAM admission for dataset generations.

use std::{error::Error, fmt};

use crate::DatasetGeneration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceReservation {
    pub ram_bytes: u64,
    pub vram_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_ram_bytes: u64,
    pub max_vram_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingResourceReservation {
    pub generation: DatasetGeneration,
    pub reservation: ResourceReservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    RamBudgetExceeded {
        requested_bytes: u64,
        max_bytes: u64,
    },
    VramBudgetExceeded {
        requested_bytes: u64,
        max_bytes: u64,
    },
    StalePendingGeneration,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RamBudgetExceeded {
                requested_bytes,
                max_bytes,
            } => write!(
                formatter,
                "dataset generation requires {requested_bytes} RAM bytes, exceeding {max_bytes}"
            ),
            Self::VramBudgetExceeded {
                requested_bytes,
                max_bytes,
            } => write!(
                formatter,
                "dataset generation requires {requested_bytes} VRAM bytes, exceeding {max_bytes}"
            ),
            Self::StalePendingGeneration => {
                formatter.write_str("dataset generation admission completion is stale")
            }
        }
    }
}

impl Error for AdmissionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetGenerationAdmission {
    budget: ResourceBudget,
    active: Option<PendingResourceReservation>,
    pending: Option<PendingResourceReservation>,
}

impl DatasetGenerationAdmission {
    pub const fn new(budget: ResourceBudget) -> Self {
        Self {
            budget,
            active: None,
            pending: None,
        }
    }

    pub const fn active(&self) -> Option<PendingResourceReservation> {
        self.active
    }
    pub const fn pending(&self) -> Option<PendingResourceReservation> {
        self.pending
    }

    pub fn reserve(
        &mut self,
        generation: DatasetGeneration,
        reservation: ResourceReservation,
    ) -> Result<PendingResourceReservation, AdmissionError> {
        let current = self.active.map_or(
            ResourceReservation {
                ram_bytes: 0,
                vram_bytes: 0,
            },
            |value| value.reservation,
        );
        let requested_ram = current.ram_bytes.saturating_add(reservation.ram_bytes);
        if requested_ram > self.budget.max_ram_bytes {
            return Err(AdmissionError::RamBudgetExceeded {
                requested_bytes: requested_ram,
                max_bytes: self.budget.max_ram_bytes,
            });
        }
        let requested_vram = current.vram_bytes.saturating_add(reservation.vram_bytes);
        if requested_vram > self.budget.max_vram_bytes {
            return Err(AdmissionError::VramBudgetExceeded {
                requested_bytes: requested_vram,
                max_bytes: self.budget.max_vram_bytes,
            });
        }
        let pending = PendingResourceReservation {
            generation,
            reservation,
        };
        self.pending = Some(pending);
        Ok(pending)
    }

    pub fn commit(&mut self, pending: PendingResourceReservation) -> Result<(), AdmissionError> {
        if self.pending != Some(pending) {
            return Err(AdmissionError::StalePendingGeneration);
        }
        self.pending = None;
        self.active = Some(pending);
        Ok(())
    }

    pub fn cancel(&mut self, pending: PendingResourceReservation) -> bool {
        if self.pending == Some(pending) {
            self.pending = None;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatasetGenerationCounter;

    #[test]
    fn replacement_is_admitted_without_clearing_active_generation() {
        let mut generations = DatasetGenerationCounter::default();
        let first = generations.mint();
        let second = generations.mint();
        let mut admission = DatasetGenerationAdmission::new(ResourceBudget {
            max_ram_bytes: 30,
            max_vram_bytes: 30,
        });
        let active = admission
            .reserve(
                first,
                ResourceReservation {
                    ram_bytes: 10,
                    vram_bytes: 8,
                },
            )
            .unwrap();
        admission.commit(active).unwrap();
        let pending = admission
            .reserve(
                second,
                ResourceReservation {
                    ram_bytes: 12,
                    vram_bytes: 9,
                },
            )
            .unwrap();
        assert_eq!(admission.active(), Some(active));
        admission.commit(pending).unwrap();
        assert_eq!(admission.active(), Some(pending));
    }

    #[test]
    fn over_budget_replacement_keeps_active_generation() {
        let mut generations = DatasetGenerationCounter::default();
        let active_generation = generations.mint();
        let pending_generation = generations.mint();
        let mut admission = DatasetGenerationAdmission::new(ResourceBudget {
            max_ram_bytes: 10,
            max_vram_bytes: 10,
        });
        let active = admission
            .reserve(
                active_generation,
                ResourceReservation {
                    ram_bytes: 8,
                    vram_bytes: 1,
                },
            )
            .unwrap();
        admission.commit(active).unwrap();
        assert!(matches!(
            admission.reserve(
                pending_generation,
                ResourceReservation {
                    ram_bytes: 3,
                    vram_bytes: 1
                }
            ),
            Err(AdmissionError::RamBudgetExceeded { .. })
        ));
        assert_eq!(admission.active(), Some(active));
    }
}
