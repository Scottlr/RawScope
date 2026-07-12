//! Timeline-density brush geometry and CPU-side selected-event summaries.

use rawscope_core::U64Range;
use rawscope_data::{SyntheticEventType, TimelineEventRecord};
use rawscope_evidence::{SelectedEventTypeCounts, TimelineLaneRange};

use crate::{BrushScreenRect, BrushScreenSize, TimelineViewport};

/// In-progress screen-space timeline brush drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineBrushDrag {
    pub screen_rect: BrushScreenRect,
}

impl TimelineBrushDrag {
    /// Builds a timeline drag rectangle directly from an existing screen rect.
    pub fn from_screen_rect(screen_rect: BrushScreenRect) -> Self {
        Self { screen_rect }
    }

    /// Finalizes this drag into a time/lane selection for the current viewport.
    pub fn finalize(
        self,
        screen_size: BrushScreenSize,
        viewport: TimelineViewport,
    ) -> Option<TimelineBrushSelection> {
        TimelineBrushSelection::from_screen_rect(self.screen_rect, screen_size, viewport)
    }
}

/// Finalized data-space timeline brush selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineBrushSelection {
    pub time_range: U64Range,
    pub lane_range: TimelineLaneRange,
}

impl TimelineBrushSelection {
    /// Converts a screen-space rectangle into a time range and half-open lane range.
    pub fn from_screen_rect(
        screen_rect: BrushScreenRect,
        screen_size: BrushScreenSize,
        viewport: TimelineViewport,
    ) -> Option<Self> {
        let screen_has_area = screen_size.width > 0.0 && screen_size.height > 0.0;
        if !screen_has_area {
            return None;
        }

        let min_x_fraction = screen_rect.min_x / screen_size.width;
        let max_x_fraction = screen_rect.max_x / screen_size.width;
        let time_min = viewport.time_at_fraction(min_x_fraction);
        let time_max = viewport.time_at_fraction(max_x_fraction);
        let time_range = time_range_from_bounds(time_min, time_max, viewport.time_range())?;
        let lane_range = lane_range_from_screen_rect(screen_rect, screen_size, viewport)?;

        Some(Self {
            time_range,
            lane_range,
        })
    }

    /// Projects this data-space selection into the current viewport.
    ///
    /// Fully off-screen selections are hidden. Partially visible selections are clamped to the
    /// current time viewport and lane bounds.
    pub fn project_to_screen(
        self,
        viewport: TimelineViewport,
        screen_size: BrushScreenSize,
    ) -> Option<BrushScreenRect> {
        let screen_has_area = screen_size.width > 0.0 && screen_size.height > 0.0;
        if !screen_has_area {
            return None;
        }

        let visible_time_range = intersect_time_range(self.time_range, viewport.time_range())?;
        let visible_lane_range = intersect_lane_range(
            self.lane_range,
            TimelineLaneRange::new(0, viewport.lane_count()),
        )?;
        let time_span = viewport.time_range().span() as f32;
        let min_x_fraction =
            (visible_time_range.min - viewport.time_range().min) as f32 / time_span;
        let max_x_fraction =
            (visible_time_range.max - viewport.time_range().min) as f32 / time_span;
        let min_y_fraction = visible_lane_range.start as f32 / viewport.lane_count() as f32;
        let max_y_fraction = visible_lane_range.end_exclusive as f32 / viewport.lane_count() as f32;

        Some(BrushScreenRect {
            min_x: min_x_fraction.clamp(0.0, 1.0) * screen_size.width,
            min_y: min_y_fraction.clamp(0.0, 1.0) * screen_size.height,
            max_x: max_x_fraction.clamp(0.0, 1.0) * screen_size.width,
            max_y: max_y_fraction.clamp(0.0, 1.0) * screen_size.height,
        })
    }

    /// Returns true when an event lies inside this brush's time and lane ranges.
    pub fn contains_event(self, event: &TimelineEventRecord) -> bool {
        self.time_range.contains(event.timestamp) && self.lane_range.contains(event.lane)
    }
}

/// CPU-side summary for the current timeline brush selection.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSelectionSummary {
    pub selected_event_count: usize,
    pub total_event_count: usize,
    pub selected_percentage: f32,
    pub selected_time_range: U64Range,
    pub selected_lane_range: TimelineLaneRange,
    pub lane_counts: Vec<usize>,
    pub event_type_counts: SelectedEventTypeCounts,
    pub top_lane: Option<u32>,
    pub top_event_type: Option<SyntheticEventType>,
    pub selected_timestamp_range: Option<U64Range>,
    pub selected_value_range: Option<(f32, f32)>,
}

impl TimelineSelectionSummary {
    /// Summarizes timeline event records inside the given timeline brush.
    pub fn from_events(
        events: &[TimelineEventRecord],
        selection: TimelineBrushSelection,
        lane_count: u32,
    ) -> Self {
        let mut selected_event_count = 0;
        let mut selected_min_timestamp = u64::MAX;
        let mut selected_max_timestamp = 0;
        let mut selected_min_value = f32::INFINITY;
        let mut selected_max_value = f32::NEG_INFINITY;
        let mut lane_counts = vec![0; lane_count as usize];
        let mut event_type_counts = SelectedEventTypeCounts::default();

        for event in events {
            let event_is_selected = selection.contains_event(event);
            if !event_is_selected {
                continue;
            }

            selected_event_count += 1;
            selected_min_timestamp = selected_min_timestamp.min(event.timestamp);
            selected_max_timestamp = selected_max_timestamp.max(event.timestamp);
            selected_min_value = selected_min_value.min(event.value);
            selected_max_value = selected_max_value.max(event.value);
            if let Some(lane_count) = lane_counts.get_mut(event.lane as usize) {
                *lane_count += 1;
            }
            event_type_counts.add(event.kind);
        }

        let selected_percentage = if events.is_empty() {
            0.0
        } else {
            selected_event_count as f32 / events.len() as f32 * 100.0
        };
        let selected_timestamp_range = selected_time_range(
            selected_event_count,
            selected_min_timestamp,
            selected_max_timestamp,
        );
        let selected_value_range =
            selected_value_range(selected_event_count, selected_min_value, selected_max_value);
        let top_lane = top_lane(&lane_counts);
        let top_event_type = event_type_counts.top_event_type();

        Self {
            selected_event_count,
            total_event_count: events.len(),
            selected_percentage,
            selected_time_range: selection.time_range,
            selected_lane_range: selection.lane_range,
            lane_counts,
            event_type_counts,
            top_lane,
            top_event_type,
            selected_timestamp_range,
            selected_value_range,
        }
    }

    /// Summarizes finalized timeline membership from one immutable snapshot.
    pub fn from_snapshot(
        events: &[TimelineEventRecord],
        snapshot: &rawscope_analysis::selection::SelectionSnapshot,
        selection: TimelineBrushSelection,
        lane_count: u32,
    ) -> Self {
        let mut selected_event_count = 0;
        let mut selected_min_timestamp = u64::MAX;
        let mut selected_max_timestamp = 0;
        let mut selected_min_value = f32::INFINITY;
        let mut selected_max_value = f32::NEG_INFINITY;
        let mut lane_counts = vec![0; lane_count as usize];
        let mut event_type_counts = SelectedEventTypeCounts::default();
        for event in events {
            if snapshot.row_ids().binary_search(&event.row_id).is_err() {
                continue;
            }
            selected_event_count += 1;
            selected_min_timestamp = selected_min_timestamp.min(event.timestamp);
            selected_max_timestamp = selected_max_timestamp.max(event.timestamp);
            selected_min_value = selected_min_value.min(event.value);
            selected_max_value = selected_max_value.max(event.value);
            if let Some(lane_count) = lane_counts.get_mut(event.lane as usize) {
                *lane_count += 1;
            }
            event_type_counts.add(event.kind);
        }
        let selected_percentage = if events.is_empty() {
            0.0
        } else {
            selected_event_count as f32 / events.len() as f32 * 100.0
        };
        Self {
            selected_event_count,
            total_event_count: events.len(),
            selected_percentage,
            selected_time_range: selection.time_range,
            selected_lane_range: selection.lane_range,
            top_lane: top_lane(&lane_counts),
            top_event_type: event_type_counts.top_event_type(),
            lane_counts,
            event_type_counts,
            selected_timestamp_range: selected_time_range(
                selected_event_count,
                selected_min_timestamp,
                selected_max_timestamp,
            ),
            selected_value_range: selected_value_range(
                selected_event_count,
                selected_min_value,
                selected_max_value,
            ),
        }
    }
}

fn lane_range_from_screen_rect(
    screen_rect: BrushScreenRect,
    screen_size: BrushScreenSize,
    viewport: TimelineViewport,
) -> Option<TimelineLaneRange> {
    let lane_count = viewport.lane_count();
    let start_lane = ((screen_rect.min_y / screen_size.height) * lane_count as f32).floor() as u32;
    let end_lane = ((screen_rect.max_y / screen_size.height) * lane_count as f32).ceil() as u32;
    let clamped_start_lane = start_lane.min(lane_count.saturating_sub(1));
    let clamped_end_lane = end_lane.clamp(clamped_start_lane + 1, lane_count);

    Some(TimelineLaneRange::new(clamped_start_lane, clamped_end_lane))
}

fn time_range_from_bounds(min: u64, max: u64, viewport_time_range: U64Range) -> Option<U64Range> {
    if max > min {
        return Some(U64Range::new(min, max));
    }

    let expanded_max = min.saturating_add(1).min(viewport_time_range.max);
    (expanded_max > min).then(|| U64Range::new(min, expanded_max))
}

fn intersect_time_range(selection: U64Range, viewport: U64Range) -> Option<U64Range> {
    let visible_min = selection.min.max(viewport.min);
    let visible_max = selection.max.min(viewport.max);
    let range_is_visible = visible_max > visible_min;
    range_is_visible.then(|| U64Range::new(visible_min, visible_max))
}

fn intersect_lane_range(
    selection: TimelineLaneRange,
    viewport: TimelineLaneRange,
) -> Option<TimelineLaneRange> {
    let visible_start = selection.start.max(viewport.start);
    let visible_end = selection.end_exclusive.min(viewport.end_exclusive);
    let range_is_visible = visible_end > visible_start;
    range_is_visible.then(|| TimelineLaneRange::new(visible_start, visible_end))
}

fn selected_time_range(selected_event_count: usize, min: u64, max: u64) -> Option<U64Range> {
    if selected_event_count == 0 {
        return None;
    }

    Some(U64Range::from_bounds_expanded(min, max))
}

fn selected_value_range(selected_event_count: usize, min: f32, max: f32) -> Option<(f32, f32)> {
    (selected_event_count > 0).then_some((min, max))
}

fn top_lane(lane_counts: &[usize]) -> Option<u32> {
    lane_counts
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .and_then(|(lane, count)| (count > 0).then_some(lane as u32))
}
