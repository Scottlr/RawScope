//! GPU-side packing for timeline-density events.

use bytemuck::{Pod, Zeroable};
use rawscope_core::U64Range;
use rawscope_data::SyntheticEventRecord;

use crate::gpu_timeline_density::GpuTimelineDensityError;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuTimelineEvent {
    timestamp_offset: u32,
    lane: u32,
    is_before_time_range: u32,
    is_after_time_range: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct TimelineParams {
    time_span: u32,
    lane_count: u32,
    grid_width: u32,
    grid_height: u32,
    event_start: u32,
    dispatch_event_count: u32,
}

impl TimelineParams {
    pub(crate) fn new(
        time_span: u32,
        lane_count: u32,
        grid_width: u32,
        grid_height: u32,
        event_start: u32,
        dispatch_event_count: u32,
    ) -> Self {
        Self {
            time_span,
            lane_count,
            grid_width,
            grid_height,
            event_start,
            dispatch_event_count,
        }
    }
}

pub(crate) fn pack_events(
    events: &[SyntheticEventRecord],
    time_range: U64Range,
) -> Result<Vec<GpuTimelineEvent>, GpuTimelineDensityError> {
    events
        .iter()
        .map(|event| pack_event(event, time_range))
        .collect()
}

pub(crate) fn timeline_span_u32(time_range: U64Range) -> Result<u32, GpuTimelineDensityError> {
    let span = time_range.span();
    u32::try_from(span).map_err(|_| GpuTimelineDensityError::TimeRangeTooWide { span })
}

fn pack_event(
    event: &SyntheticEventRecord,
    time_range: U64Range,
) -> Result<GpuTimelineEvent, GpuTimelineDensityError> {
    let timestamp_is_before_range = event.timestamp < time_range.min;
    let timestamp_is_after_range = event.timestamp > time_range.max;
    let timestamp_offset = if timestamp_is_before_range {
        0
    } else {
        let offset = event.timestamp.saturating_sub(time_range.min);
        u32::try_from(offset)
            .map_err(|_| GpuTimelineDensityError::TimeRangeTooWide { span: offset })?
    };

    Ok(GpuTimelineEvent {
        timestamp_offset,
        lane: event.lane,
        is_before_time_range: u32::from(timestamp_is_before_range),
        is_after_time_range: u32::from(timestamp_is_after_range),
    })
}

#[cfg(test)]
mod tests {
    use rawscope_core::{RowId, U64Range};
    use rawscope_data::{SyntheticEventRecord, SyntheticEventType};

    use super::{pack_event, timeline_span_u32};
    use crate::gpu_timeline_density::GpuTimelineDensityError;

    #[test]
    fn pack_event_marks_timestamps_outside_range() {
        let time_range = U64Range::new(100, 200);
        let before = pack_event(&event(0, 99, 0), time_range).unwrap();
        let inside = pack_event(&event(1, 150, 0), time_range).unwrap();
        let after = pack_event(&event(2, 201, 0), time_range).unwrap();

        assert_eq!(before.is_before_time_range, 1);
        assert_eq!(before.is_after_time_range, 0);
        assert_eq!(inside.timestamp_offset, 50);
        assert_eq!(inside.is_before_time_range, 0);
        assert_eq!(inside.is_after_time_range, 0);
        assert_eq!(after.is_before_time_range, 0);
        assert_eq!(after.is_after_time_range, 1);
    }

    #[test]
    fn timeline_span_rejects_ranges_wider_than_u32() {
        let err = timeline_span_u32(U64Range::new(0, u32::MAX as u64 + 1))
            .expect_err("wide ranges should be rejected");

        assert!(matches!(
            err,
            GpuTimelineDensityError::TimeRangeTooWide { .. }
        ));
    }

    fn event(row_id: u64, timestamp: u64, lane: u32) -> SyntheticEventRecord {
        SyntheticEventRecord {
            row_id: RowId(row_id),
            timestamp,
            lane,
            value: 1.0,
            event_type: SyntheticEventType::Background,
        }
    }
}
