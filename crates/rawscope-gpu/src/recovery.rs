//! Device-generation recovery state independent of renderer workflow.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceLossReason {
    Unknown,
    Destroyed,
    OutOfMemory,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRecoveryState {
    Ready(DeviceGeneration),
    SurfaceOutdated {
        generation: DeviceGeneration,
    },
    DeviceLost {
        generation: DeviceGeneration,
        reason: DeviceLossReason,
    },
    Recovering {
        previous: DeviceGeneration,
    },
    Fatal,
}

impl GpuRecoveryState {
    pub fn accepts(self, generation: DeviceGeneration) -> bool {
        matches!(self, Self::Ready(current) if current == generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_generations_cannot_publish_after_loss() {
        let generation = DeviceGeneration(4);
        assert!(GpuRecoveryState::Ready(generation).accepts(generation));
        assert!(!GpuRecoveryState::DeviceLost {
            generation,
            reason: DeviceLossReason::Destroyed
        }
        .accepts(generation));
    }
}
