//! Generation-safe resident timeline field lifecycle.

use rawscope_core::U64Range;
use rawscope_data::DatasetGeneration;
use rawscope_gpu::DeviceGeneration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineUpdateQuality {
    Preview,
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineFieldGeneration(u64);

impl TimelineFieldGeneration {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineUpdateRequest {
    pub time_range: U64Range,
    pub lane_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub quality: TimelineUpdateQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineFieldState {
    pub generation: TimelineFieldGeneration,
    pub request: TimelineUpdateRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTimelineField {
    pub generation: TimelineFieldGeneration,
    pub dataset_generation: DatasetGeneration,
    pub device_generation: DeviceGeneration,
    pub request: TimelineUpdateRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineResidentError {
    ZeroLaneCount,
    ZeroGridDimension,
}

impl std::fmt::Display for TimelineResidentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ZeroLaneCount => "timeline resident state requires at least one lane",
            Self::ZeroGridDimension => "timeline resident state requires positive grid dimensions",
        })
    }
}

impl std::error::Error for TimelineResidentError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineResidentState {
    dataset_generation: DatasetGeneration,
    device_generation: DeviceGeneration,
    resident_event_count: u64,
    next_generation: u64,
    active: Option<TimelineFieldState>,
    pending: Option<PendingTimelineField>,
}

impl TimelineResidentState {
    pub fn new(
        dataset_generation: DatasetGeneration,
        device_generation: DeviceGeneration,
        resident_event_count: u64,
    ) -> Self {
        Self {
            dataset_generation,
            device_generation,
            resident_event_count,
            next_generation: 0,
            active: None,
            pending: None,
        }
    }

    pub const fn dataset_generation(&self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub const fn device_generation(&self) -> DeviceGeneration {
        self.device_generation
    }

    pub const fn resident_event_count(&self) -> u64 {
        self.resident_event_count
    }

    pub const fn active(&self) -> Option<TimelineFieldState> {
        self.active
    }

    pub const fn pending(&self) -> Option<PendingTimelineField> {
        self.pending
    }

    pub fn requires_event_upload(
        &self,
        dataset_generation: DatasetGeneration,
        event_count: u64,
    ) -> bool {
        self.dataset_generation != dataset_generation || self.resident_event_count != event_count
    }

    pub fn submit(
        &mut self,
        request: TimelineUpdateRequest,
    ) -> Result<PendingTimelineField, TimelineResidentError> {
        validate_request(request)?;
        self.next_generation = self.next_generation.saturating_add(1).max(1);
        let pending = PendingTimelineField {
            generation: TimelineFieldGeneration(self.next_generation),
            dataset_generation: self.dataset_generation,
            device_generation: self.device_generation,
            request,
        };
        self.pending = Some(pending);
        Ok(pending)
    }

    pub fn complete(&mut self, pending: PendingTimelineField) -> bool {
        if self.pending != Some(pending)
            || pending.dataset_generation != self.dataset_generation
            || pending.device_generation != self.device_generation
        {
            return false;
        }
        self.pending = None;
        self.active = Some(TimelineFieldState {
            generation: pending.generation,
            request: pending.request,
        });
        true
    }

    pub fn cancel(&mut self, generation: TimelineFieldGeneration) -> bool {
        if self
            .pending
            .is_some_and(|pending| pending.generation == generation)
        {
            self.pending = None;
            true
        } else {
            false
        }
    }

    pub fn replace_dataset(
        &mut self,
        dataset_generation: DatasetGeneration,
        device_generation: DeviceGeneration,
        event_count: u64,
    ) {
        self.dataset_generation = dataset_generation;
        self.device_generation = device_generation;
        self.resident_event_count = event_count;
        self.active = None;
        self.pending = None;
    }
}

fn validate_request(request: TimelineUpdateRequest) -> Result<(), TimelineResidentError> {
    if request.lane_count == 0 {
        return Err(TimelineResidentError::ZeroLaneCount);
    }
    if request.grid_width == 0 || request.grid_height == 0 {
        return Err(TimelineResidentError::ZeroGridDimension);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_core::GenerationCounter;
    use rawscope_data::DatasetStoreOwner;

    fn generations() -> (DatasetGeneration, DeviceGeneration) {
        let mut counter = GenerationCounter::<DatasetStoreOwner>::default();
        (counter.mint(), DeviceGeneration(4))
    }

    fn request(quality: TimelineUpdateQuality) -> TimelineUpdateRequest {
        TimelineUpdateRequest {
            time_range: U64Range::new(10, 20),
            lane_count: 2,
            grid_width: 16,
            grid_height: 8,
            quality,
        }
    }

    #[test]
    fn resident_state_reuses_events_and_publishes_only_completed_fields() {
        let (dataset_generation, device_generation) = generations();
        let mut state = TimelineResidentState::new(dataset_generation, device_generation, 100);
        assert!(!state.requires_event_upload(dataset_generation, 100));
        let pending = state
            .submit(request(TimelineUpdateQuality::Preview))
            .unwrap();
        assert!(state.active().is_none());
        assert!(state.complete(pending));
        assert_eq!(state.active().unwrap().generation, pending.generation);
    }

    #[test]
    fn stale_completion_and_cancelled_preview_preserve_active_field() {
        let (dataset_generation, device_generation) = generations();
        let mut state = TimelineResidentState::new(dataset_generation, device_generation, 2);
        let first = state.submit(request(TimelineUpdateQuality::Exact)).unwrap();
        assert!(state.complete(first));
        let active = state.active();
        let second = state
            .submit(request(TimelineUpdateQuality::Preview))
            .unwrap();
        assert!(state.cancel(second.generation));
        assert!(!state.complete(second));
        assert_eq!(state.active(), active);
    }

    #[test]
    fn request_requires_positive_dimensions() {
        let (dataset_generation, device_generation) = generations();
        let mut state = TimelineResidentState::new(dataset_generation, device_generation, 1);
        assert_eq!(
            state.submit(TimelineUpdateRequest {
                lane_count: 0,
                ..request(TimelineUpdateQuality::Exact)
            }),
            Err(TimelineResidentError::ZeroLaneCount)
        );
    }
}
