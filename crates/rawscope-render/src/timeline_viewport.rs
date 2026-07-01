//! Testable timeline-density viewport math for time-axis pan, zoom, and reset.

use rawscope_core::U64Range;

const MIN_TIMELINE_SPAN: u64 = 1;
const MIN_ZOOM_SCALE: f32 = 0.05;
const MAX_ZOOM_SCALE: f32 = 20.0;

/// Current timeline-density time viewport constrained by the full synthetic range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineViewport {
    full_time_range: U64Range,
    time_range: U64Range,
    lane_count: u32,
    min_span: u64,
}

impl TimelineViewport {
    /// Creates a viewport showing the full time range with a stable lane count.
    pub fn new(full_time_range: U64Range, lane_count: u32) -> Self {
        assert!(lane_count > 0, "lane_count must be positive");

        Self {
            full_time_range,
            time_range: full_time_range,
            lane_count,
            min_span: MIN_TIMELINE_SPAN,
        }
    }

    /// Returns the immutable full synthetic time range.
    pub fn full_time_range(self) -> U64Range {
        self.full_time_range
    }

    /// Returns the current visible time range.
    pub fn time_range(self) -> U64Range {
        self.time_range
    }

    /// Returns the stable lane count for this timeline dataset.
    pub fn lane_count(self) -> u32 {
        self.lane_count
    }

    /// Resets the visible time range back to the full synthetic range.
    pub fn reset(&mut self) {
        self.time_range = self.full_time_range;
    }

    /// Zooms the visible time range around a normalized x-axis anchor.
    pub fn zoom_around_fraction(&mut self, anchor_fraction: f32, scale: f32) {
        let clamped_anchor_fraction = anchor_fraction.clamp(0.0, 1.0);
        let clamped_scale = scale.clamp(MIN_ZOOM_SCALE, MAX_ZOOM_SCALE);
        let current_span = self.time_range.span();
        let target_span = ((current_span as f64) * (clamped_scale as f64)).round() as u64;
        let clamped_target_span = target_span.clamp(self.min_span, self.full_time_range.span());
        let anchor_time = self.time_at_fraction(clamped_anchor_fraction);
        let left_span = ((clamped_target_span as f64) * (clamped_anchor_fraction as f64)).round();
        let target_min = (anchor_time as i128) - (left_span as i128);

        self.time_range =
            clamp_range_to_full(target_min, clamped_target_span, self.full_time_range);
    }

    /// Pans the visible time range by an integer time delta.
    pub fn pan_by(&mut self, delta: i64) {
        let target_min = (self.time_range.min as i128) + (delta as i128);
        self.time_range =
            clamp_range_to_full(target_min, self.time_range.span(), self.full_time_range);
    }

    fn time_at_fraction(self, fraction: f32) -> u64 {
        let clamped_fraction = fraction.clamp(0.0, 1.0);
        let offset = ((self.time_range.span() as f64) * (clamped_fraction as f64)).round() as u64;
        self.time_range
            .min
            .saturating_add(offset)
            .min(self.time_range.max)
    }
}

fn clamp_range_to_full(target_min: i128, target_span: u64, full: U64Range) -> U64Range {
    let clamped_span = target_span.clamp(MIN_TIMELINE_SPAN, full.span());
    if clamped_span >= full.span() {
        return full;
    }

    let full_min = full.min as i128;
    let max_min = (full.max - clamped_span) as i128;
    let clamped_min = target_min.clamp(full_min, max_min) as u64;

    U64Range::new(clamped_min, clamped_min + clamped_span)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> TimelineViewport {
        TimelineViewport::new(U64Range::new(1_000, 2_000), 8)
    }

    #[test]
    fn reset_returns_to_full_range() {
        let mut viewport = viewport();
        viewport.zoom_around_fraction(0.5, 0.5);
        viewport.pan_by(100);

        viewport.reset();

        assert_eq!(viewport.time_range(), viewport.full_time_range());
    }

    #[test]
    fn zoom_reduces_span_around_anchor() {
        let mut viewport = viewport();

        viewport.zoom_around_fraction(0.5, 0.5);

        assert_eq!(viewport.time_range(), U64Range::new(1_250, 1_750));
    }

    #[test]
    fn pan_shifts_range() {
        let mut viewport = viewport();
        viewport.zoom_around_fraction(0.5, 0.5);

        viewport.pan_by(100);

        assert_eq!(viewport.time_range(), U64Range::new(1_350, 1_850));
    }

    #[test]
    fn pan_clamps_to_bounds() {
        let mut viewport = viewport();
        viewport.zoom_around_fraction(0.5, 0.5);

        viewport.pan_by(-1_000);
        assert_eq!(viewport.time_range(), U64Range::new(1_000, 1_500));

        viewport.pan_by(1_000);
        assert_eq!(viewport.time_range(), U64Range::new(1_500, 2_000));
    }

    #[test]
    fn repeated_zoom_does_not_invert_or_zero_range() {
        let mut viewport = viewport();

        for _ in 0..64 {
            viewport.zoom_around_fraction(0.5, 0.01);
        }

        assert!(viewport.time_range().span() >= MIN_TIMELINE_SPAN);
        assert!(viewport.time_range().max > viewport.time_range().min);
    }

    #[test]
    fn minimum_span_is_respected() {
        let mut viewport = TimelineViewport::new(U64Range::new(10, 12), 4);

        viewport.zoom_around_fraction(0.5, 0.01);

        assert_eq!(viewport.time_range().span(), MIN_TIMELINE_SPAN);
    }

    #[test]
    fn lane_count_remains_stable_after_view_changes() {
        let mut viewport = viewport();

        viewport.zoom_around_fraction(0.25, 0.5);
        viewport.pan_by(75);
        viewport.reset();

        assert_eq!(viewport.lane_count(), 8);
    }
}
