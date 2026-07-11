use super::*;

fn rect(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Rect {
    Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y))
}

fn placement(bin_rect: Rect, viewport_rect: Rect) -> TooltipPlacement {
    place_tooltip(TooltipPlacementInput {
        bin_rect,
        tooltip_size: vec2(80.0, 50.0),
        viewport_rect,
        gap_px: 10.0,
    })
}

#[test]
fn tooltip_prefers_right_then_flips_at_viewport_edge() {
    let viewport = rect(0.0, 0.0, 400.0, 240.0);
    assert_eq!(
        placement(rect(100.0, 90.0, 120.0, 110.0), viewport).side,
        TooltipSide::Right
    );
    assert_eq!(
        placement(rect(320.0, 90.0, 340.0, 110.0), viewport).side,
        TooltipSide::Left
    );
}

#[test]
fn tooltip_clamps_without_covering_inspected_bin() {
    let viewport = rect(0.0, 0.0, 240.0, 160.0);
    let bin = rect(200.0, 130.0, 220.0, 150.0);
    let result = placement(bin, viewport);

    assert!(viewport.contains_rect(result.rect));
    assert!(!result.rect.intersects(bin));
}

#[test]
fn tooltip_motion_finishes_and_stops_requesting_frames() {
    let key = InspectionPresentationKey {
        viewport_revision: 1,
        filter_revision: Default::default(),
        bin_x: 2,
        bin_y: 3,
        density_mode: rawscope_render::ScatterDensityMode::AbsoluteDensity,
    };
    let mut motion = InspectionPresentationMotion::default();
    motion.sync(Some(key), 10, false);
    assert!(motion.frame(10, false).running);
    assert!(!motion.frame(10 + TOOLTIP_ENTER_DURATION_MS, false).running);
    motion.sync(None, 10 + TOOLTIP_ENTER_DURATION_MS, false);
    assert!(
        motion
            .frame(10 + TOOLTIP_ENTER_DURATION_MS + 1, false)
            .running
    );
    assert!(
        !motion
            .frame(
                10 + TOOLTIP_ENTER_DURATION_MS + TOOLTIP_EXIT_DURATION_MS,
                false
            )
            .running
    );
}

#[test]
fn reduced_motion_is_immediate() {
    let key = InspectionPresentationKey {
        viewport_revision: 1,
        filter_revision: Default::default(),
        bin_x: 0,
        bin_y: 0,
        density_mode: rawscope_render::ScatterDensityMode::AbsoluteDensity,
    };
    let mut motion = InspectionPresentationMotion::default();
    motion.sync(Some(key), 100, true);
    assert_eq!(
        motion.frame(100, true),
        InspectionPresentationFrame {
            opacity: 1.0,
            translate_y_px: 0.0,
            running: false,
        }
    );
    motion.sync(None, 101, true);
    assert_eq!(motion.frame(101, true).opacity, 0.0);
}
