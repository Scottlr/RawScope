use rawscope_core::U64Range;
use rawscope_render::TimelineViewport;

fn viewport() -> TimelineViewport {
    TimelineViewport::new(U64Range::new(1_000, 2_000), 8)
}

#[test]
fn reset_returns_to_full_range() {
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.5, 0.5);
    viewport.pan_by(100);

    viewport.reset();

    assert_eq!(viewport.current_time_range(), viewport.full_time_range());
}

#[test]
fn zoom_reduces_span() {
    let mut viewport = viewport();

    viewport.zoom_around_fraction(0.5, 0.5);

    assert_eq!(viewport.current_time_range().span(), 500);
}

#[test]
fn zoom_around_cursor_anchor_is_deterministic() {
    let mut viewport = viewport();

    viewport.zoom_around_fraction(0.25, 0.5);

    assert_eq!(viewport.current_time_range(), U64Range::new(1_125, 1_625));
    assert_eq!(viewport.time_at_fraction(0.25), 1_250);
}

#[test]
fn pan_shifts_range() {
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.5, 0.5);

    viewport.pan_by(100);

    assert_eq!(viewport.current_time_range(), U64Range::new(1_350, 1_850));
}

#[test]
fn screen_fraction_pan_shifts_range_like_mouse_drag() {
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.5, 0.5);

    viewport.pan_by_screen_fraction(0.2);

    assert_eq!(viewport.current_time_range(), U64Range::new(1_150, 1_650));
}

#[test]
fn pan_clamps_to_bounds() {
    let mut viewport = viewport();
    viewport.zoom_around_fraction(0.5, 0.5);

    viewport.pan_by(-1_000);
    assert_eq!(viewport.current_time_range(), U64Range::new(1_000, 1_500));

    viewport.pan_by(1_000);
    assert_eq!(viewport.current_time_range(), U64Range::new(1_500, 2_000));
}

#[test]
fn minimum_span_is_respected() {
    let mut viewport = TimelineViewport::new(U64Range::new(10, 12), 4);

    viewport.zoom_around_fraction(0.5, 0.01);

    assert_eq!(
        viewport.current_time_range().span(),
        viewport.minimum_span()
    );
}

#[test]
fn repeated_zoom_never_inverts_or_creates_zero_width_range() {
    let mut viewport = viewport();

    for _ in 0..64 {
        viewport.zoom_around_fraction(0.5, 0.01);
    }

    assert!(viewport.current_time_range().span() >= viewport.minimum_span());
    assert!(viewport.current_time_range().max > viewport.current_time_range().min);
}

#[test]
fn lane_count_remains_stable() {
    let mut viewport = viewport();

    viewport.zoom_around_fraction(0.25, 0.5);
    viewport.pan_by(75);
    viewport.reset();

    assert_eq!(viewport.lane_count(), 8);
}

#[test]
fn time_at_fraction_clamps_to_current_time_range() {
    let viewport = viewport();

    assert_eq!(viewport.time_at_fraction(-1.0), 1_000);
    assert_eq!(viewport.time_at_fraction(0.5), 1_500);
    assert_eq!(viewport.time_at_fraction(2.0), 2_000);
}
