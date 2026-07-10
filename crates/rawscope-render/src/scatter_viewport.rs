//! Testable scatter-density viewport math for pan, zoom, and reset.

use rawscope_core::F32Range;

const MIN_VIEWPORT_FRACTION: f32 = 0.0001;
const MIN_ZOOM_SCALE: f32 = 0.05;
const MAX_ZOOM_SCALE: f32 = 20.0;
const MAX_PAN_OVERSCROLL_FRACTION: f32 = 0.2;

/// Current scatter-density data viewport with bounded movement around full data ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterViewport {
    full_x_range: F32Range,
    full_y_range: F32Range,
    x_range: F32Range,
    y_range: F32Range,
}

impl ScatterViewport {
    /// Creates a viewport showing the full data range.
    pub fn new(full_x_range: F32Range, full_y_range: F32Range) -> Self {
        Self {
            full_x_range,
            full_y_range,
            x_range: full_x_range,
            y_range: full_y_range,
        }
    }

    /// Returns the immutable full x range.
    pub fn full_x_range(self) -> F32Range {
        self.full_x_range
    }

    /// Returns the immutable full y range.
    pub fn full_y_range(self) -> F32Range {
        self.full_y_range
    }

    /// Returns the current x viewport.
    pub fn x_range(self) -> F32Range {
        self.x_range
    }

    /// Returns the current y viewport.
    pub fn y_range(self) -> F32Range {
        self.y_range
    }

    /// Resets the viewport back to the full data range.
    pub fn reset(&mut self) {
        self.x_range = self.full_x_range;
        self.y_range = self.full_y_range;
    }

    /// Zooms around a data-space anchor point.
    pub fn zoom_around(&mut self, anchor_x: f32, anchor_y: f32, scale: f32) {
        let clamped_scale = scale.clamp(MIN_ZOOM_SCALE, MAX_ZOOM_SCALE);
        self.x_range = zoom_range(self.x_range, self.full_x_range, anchor_x, clamped_scale);
        self.y_range = zoom_range(self.y_range, self.full_y_range, anchor_y, clamped_scale);
    }

    /// Pans the current viewport by data-space deltas.
    pub fn pan_by(&mut self, delta_x: f32, delta_y: f32) {
        self.x_range = translate_range(self.x_range, self.full_x_range, delta_x);
        self.y_range = translate_range(self.y_range, self.full_y_range, delta_y);
    }

    /// Maps normalized view coordinates to data coordinates.
    ///
    /// `x_fraction` and `y_fraction` are screen-space fractions where y grows downward.
    pub fn data_point_at_fraction(self, x_fraction: f32, y_fraction: f32) -> (f32, f32) {
        let clamped_x_fraction = x_fraction.clamp(0.0, 1.0);
        let clamped_y_fraction = y_fraction.clamp(0.0, 1.0);
        let x = self.x_range.min + self.x_range.span() * clamped_x_fraction;
        let y = self.y_range.max - self.y_range.span() * clamped_y_fraction;

        (x, y)
    }
}

fn zoom_range(current: F32Range, full: F32Range, anchor: f32, scale: f32) -> F32Range {
    let min_span = full.span() * MIN_VIEWPORT_FRACTION;
    let target_span = (current.span() * scale).clamp(min_span, full.span());
    let anchor_fraction = ((anchor - current.min) / current.span()).clamp(0.0, 1.0);
    let target_min = anchor - target_span * anchor_fraction;

    clamp_range_to_full(target_min, target_span, full)
}

fn translate_range(current: F32Range, full: F32Range, delta: f32) -> F32Range {
    clamp_range_to_pan_bounds(current.min + delta, current.span(), full)
}

fn clamp_range_to_pan_bounds(target_min: f32, target_span: f32, full: F32Range) -> F32Range {
    let max_overscroll = target_span * MAX_PAN_OVERSCROLL_FRACTION;
    let min_target = full.min - max_overscroll;
    let max_target = full.max - target_span + max_overscroll;
    let clamped_min = target_min.clamp(min_target, max_target);

    F32Range::new(clamped_min, clamped_min + target_span)
}

fn clamp_range_to_full(target_min: f32, target_span: f32, full: F32Range) -> F32Range {
    if target_span >= full.span() {
        return full;
    }

    let max_min = full.max - target_span;
    let clamped_min = target_min.clamp(full.min, max_min);
    F32Range::new(clamped_min, clamped_min + target_span)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> ScatterViewport {
        ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 200.0))
    }

    #[test]
    fn reset_returns_to_full_range() {
        let mut viewport = viewport();
        viewport.zoom_around(50.0, 100.0, 0.5);
        viewport.pan_by(10.0, 20.0);

        viewport.reset();

        assert_eq!(viewport.x_range(), viewport.full_x_range());
        assert_eq!(viewport.y_range(), viewport.full_y_range());
    }

    #[test]
    fn zoom_reduces_range_around_anchor() {
        let mut viewport = viewport();

        viewport.zoom_around(50.0, 100.0, 0.5);

        assert_eq!(viewport.x_range(), F32Range::new(25.0, 75.0));
        assert_eq!(viewport.y_range(), F32Range::new(50.0, 150.0));
    }

    #[test]
    fn pan_shifts_range_with_bounded_overscroll() {
        let mut viewport = viewport();
        viewport.zoom_around(50.0, 100.0, 0.5);

        viewport.pan_by(80.0, -120.0);

        assert_eq!(viewport.x_range(), F32Range::new(60.0, 110.0));
        assert_eq!(viewport.y_range(), F32Range::new(-20.0, 80.0));
    }

    #[test]
    fn pan_moves_a_full_extent_view_without_losing_the_dataset() {
        let mut viewport = viewport();

        viewport.pan_by(100.0, -200.0);

        assert_eq!(viewport.x_range(), F32Range::new(20.0, 120.0));
        assert_eq!(viewport.y_range(), F32Range::new(-40.0, 160.0));
    }

    #[test]
    fn repeated_zoom_does_not_invert_or_zero_the_range() {
        let mut viewport = viewport();

        for _ in 0..64 {
            viewport.zoom_around(50.0, 100.0, 0.01);
        }

        assert!(viewport.x_range().span() > 0.0);
        assert!(viewport.y_range().span() > 0.0);
        let tolerance = 0.00001;
        let min_x_span = viewport.full_x_range().span() * MIN_VIEWPORT_FRACTION;
        let min_y_span = viewport.full_y_range().span() * MIN_VIEWPORT_FRACTION;
        assert!(viewport.x_range().span() + tolerance >= min_x_span);
        assert!(viewport.y_range().span() + tolerance >= min_y_span);
    }

    #[test]
    fn data_point_at_fraction_maps_screen_y_down_to_data_y_up() {
        let viewport = viewport();

        assert_eq!(viewport.data_point_at_fraction(0.25, 0.25), (25.0, 150.0));
    }
}
