//! Device-generation recovery state independent of renderer workflow.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceGeneration(pub u64);

impl DeviceGeneration {
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceLossReason {
    Unknown,
    Destroyed,
    OutOfMemory,
    Other,
}

impl DeviceLossReason {
    pub const fn from_wgpu(reason: wgpu::DeviceLostReason) -> Self {
        match reason {
            wgpu::DeviceLostReason::Destroyed => Self::Destroyed,
            wgpu::DeviceLostReason::Unknown => Self::Unknown,
        }
    }
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

    pub const fn surface_outdated(self) -> Self {
        match self {
            Self::Ready(generation) => Self::SurfaceOutdated { generation },
            state => state,
        }
    }

    pub const fn device_lost(self, reason: DeviceLossReason) -> Self {
        match self {
            Self::Ready(generation) | Self::SurfaceOutdated { generation } => {
                Self::DeviceLost { generation, reason }
            }
            state => state,
        }
    }

    pub const fn surface_reconfigured(self) -> Self {
        match self {
            Self::SurfaceOutdated { generation } => Self::Ready(generation),
            state => state,
        }
    }

    pub const fn begin_recovery(self) -> Self {
        match self {
            Self::DeviceLost { generation, .. } => Self::Recovering {
                previous: generation,
            },
            state => state,
        }
    }

    pub const fn recovered(self) -> Self {
        match self {
            Self::Recovering { previous } => Self::Ready(previous.next()),
            state => state,
        }
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

    #[test]
    fn recovery_requires_a_complete_generation_transition() {
        let state = GpuRecoveryState::Ready(DeviceGeneration(4));
        let state = state
            .device_lost(DeviceLossReason::Destroyed)
            .begin_recovery();
        assert_eq!(
            state.recovered(),
            GpuRecoveryState::Ready(DeviceGeneration(5))
        );
        assert!(!state.recovered().accepts(DeviceGeneration(4)));
    }

    #[test]
    fn surface_reconfiguration_preserves_device_generation() {
        let generation = DeviceGeneration(7);
        let state = GpuRecoveryState::Ready(generation)
            .surface_outdated()
            .surface_reconfigured();
        assert_eq!(state, GpuRecoveryState::Ready(generation));
    }

    #[test]
    fn wgpu_loss_reasons_are_preserved_for_recovery() {
        assert_eq!(
            DeviceLossReason::from_wgpu(wgpu::DeviceLostReason::Destroyed),
            DeviceLossReason::Destroyed
        );
        assert_eq!(
            DeviceLossReason::from_wgpu(wgpu::DeviceLostReason::Unknown),
            DeviceLossReason::Unknown
        );
    }
}
