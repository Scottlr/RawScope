use std::sync::Arc;

use rawscope_core::{F32Range, RowId};

use super::*;

#[test]
fn inspection_overlay_params_match_wgsl_uniform_alignment() {
    assert_eq!(std::mem::size_of::<InspectionOverlayParams>(), 80);
    assert_eq!(std::mem::align_of::<InspectionOverlayParams>(), 4);
    assert_eq!(std::mem::size_of::<InspectionOverlayParams>() % 16, 0);
}

fn hit(bin_x: u32, bin_y: u32) -> ScatterInspectionHit {
    ScatterInspectionHit {
        bin_x,
        bin_y,
        x_range: F32Range::new(0.0, 1.0),
        y_range: F32Range::new(0.0, 1.0),
        count: 1,
        row_ids: Arc::from([RowId(0)]),
    }
}

#[test]
fn inspection_overlay_projects_exact_and_neighborhood_rects() {
    let overlay = project_scatter_inspection_overlay(
        &hit(2, 2),
        4,
        4,
        BrushScreenSize::new(400.0, 200.0),
        InspectionFocusKind::Hover,
        1.0,
    )
    .unwrap();

    assert_eq!(
        overlay.exact_rect,
        BrushScreenRect {
            min_x: 200.0,
            min_y: 50.0,
            max_x: 300.0,
            max_y: 100.0,
        }
    );
    assert_eq!(
        overlay.neighborhood_rect,
        BrushScreenRect {
            min_x: 100.0,
            min_y: 0.0,
            max_x: 400.0,
            max_y: 150.0,
        }
    );
}

#[test]
fn inspection_overlay_clips_neighborhood_at_grid_edges() {
    let overlay = project_scatter_inspection_overlay(
        &hit(0, 3),
        4,
        4,
        BrushScreenSize::new(400.0, 200.0),
        InspectionFocusKind::Pinned,
        1.0,
    )
    .unwrap();

    assert_eq!(overlay.neighborhood_rect.min_x, 0.0);
    assert_eq!(overlay.neighborhood_rect.max_x, 200.0);
    assert_eq!(overlay.neighborhood_rect.min_y, 0.0);
    assert_eq!(overlay.neighborhood_rect.max_y, 100.0);
}

#[test]
fn inspection_overlay_preserves_top_left_screen_orientation() {
    let top_left = project_scatter_inspection_overlay(
        &hit(0, 3),
        4,
        4,
        BrushScreenSize::new(400.0, 200.0),
        InspectionFocusKind::Hover,
        1.0,
    )
    .unwrap();
    let bottom_right = project_scatter_inspection_overlay(
        &hit(3, 0),
        4,
        4,
        BrushScreenSize::new(400.0, 200.0),
        InspectionFocusKind::Hover,
        1.0,
    )
    .unwrap();

    assert_eq!(top_left.exact_rect.min_y, 0.0);
    assert_eq!(bottom_right.exact_rect.max_y, 200.0);
    assert!(top_left.exact_rect.min_x < bottom_right.exact_rect.min_x);
}

#[test]
fn inspection_overlay_params_clamp_alpha_and_add_plot_origin() {
    let overlay = project_scatter_inspection_overlay(
        &hit(0, 0),
        1,
        1,
        BrushScreenSize::new(100.0, 60.0),
        InspectionFocusKind::Pinned,
        4.0,
    )
    .unwrap();
    let plot = PlotRectPx::try_new(20, 30, 100, 60, 200, 120).unwrap();
    let params = InspectionOverlayParams::from_overlay(overlay, plot).unwrap();

    assert_eq!(params.presentation_alpha, 1.0);
    assert_eq!(params.plot_origin_px, [20.0, 30.0]);
    assert_eq!(params.plot_size_px, [100.0, 60.0]);
    assert_eq!(params.focus_kind, 1);
}

#[test]
fn hover_and_pin_use_distinct_focus_kind_uniforms() {
    let plot = PlotRectPx::try_new(0, 0, 100, 100, 100, 100).unwrap();
    let hover = project_scatter_inspection_overlay(
        &hit(0, 0),
        1,
        1,
        BrushScreenSize::new(100.0, 100.0),
        InspectionFocusKind::Hover,
        1.0,
    )
    .unwrap();
    let pinned = ScatterInspectionOverlay {
        focus_kind: InspectionFocusKind::Pinned,
        ..hover
    };

    assert_ne!(
        InspectionOverlayParams::from_overlay(hover, plot)
            .unwrap()
            .focus_kind,
        InspectionOverlayParams::from_overlay(pinned, plot)
            .unwrap()
            .focus_kind
    );
}
