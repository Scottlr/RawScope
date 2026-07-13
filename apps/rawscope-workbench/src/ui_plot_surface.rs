//! Authoritative egui allocation for the native WGPU plot surface.

use egui::{Rect, Sense, Ui};
use rawscope_render::PlotRectPx;

pub(crate) const PLOT_LEFT_GUTTER_POINTS: f32 = 58.0;
pub(crate) const PLOT_BOTTOM_GUTTER_POINTS: f32 = 38.0;
pub(crate) const PLOT_TOP_GUTTER_POINTS: f32 = 10.0;
pub(crate) const PLOT_RIGHT_GUTTER_POINTS: f32 = 12.0;

/// Stable axis gutters and the inset data rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlotAxisLayout {
    pub(crate) outer_rect: Rect,
    pub(crate) plot_rect: Rect,
}

impl PlotAxisLayout {
    fn from_outer_rect(outer_rect: Rect) -> Option<Self> {
        let plot_rect = Rect::from_min_max(
            egui::pos2(
                outer_rect.min.x + PLOT_LEFT_GUTTER_POINTS,
                outer_rect.min.y + PLOT_TOP_GUTTER_POINTS,
            ),
            egui::pos2(
                outer_rect.max.x - PLOT_RIGHT_GUTTER_POINTS,
                outer_rect.max.y - PLOT_BOTTOM_GUTTER_POINTS,
            ),
        );
        (plot_rect.width() > 0.0 && plot_rect.height() > 0.0).then_some(Self {
            outer_rect,
            plot_rect,
        })
    }
}

/// Logical and physical views of one central plot allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlotSurfaceLayout {
    pub(crate) logical_rect: Rect,
    pub(crate) physical_rect: PlotRectPx,
    pub(crate) axis_layout: PlotAxisLayout,
}

/// Allocates the remaining central UI region as the plot surface.
pub(crate) fn allocate_plot_surface(
    ui: &mut Ui,
    pixels_per_point: f32,
    surface_width_px: u32,
    surface_height_px: u32,
) -> Option<PlotSurfaceLayout> {
    let outer_rect = ui.available_rect_before_wrap();
    ui.allocate_rect(outer_rect, Sense::hover());
    let axis_layout = PlotAxisLayout::from_outer_rect(outer_rect)?;
    let logical_rect = axis_layout.plot_rect;
    let physical_rect = logical_to_physical_plot_rect(
        logical_rect,
        pixels_per_point,
        surface_width_px,
        surface_height_px,
    )?;

    Some(PlotSurfaceLayout {
        logical_rect,
        physical_rect,
        axis_layout,
    })
}

fn logical_to_physical_plot_rect(
    logical_rect: Rect,
    pixels_per_point: f32,
    surface_width_px: u32,
    surface_height_px: u32,
) -> Option<PlotRectPx> {
    let scale_is_valid = pixels_per_point.is_finite() && pixels_per_point > 0.0;
    if !scale_is_valid || !logical_rect.is_finite() {
        return None;
    }

    let surface_width = surface_width_px as f32;
    let surface_height = surface_height_px as f32;
    let min_x = (logical_rect.min.x * pixels_per_point)
        .floor()
        .clamp(0.0, surface_width) as u32;
    let min_y = (logical_rect.min.y * pixels_per_point)
        .floor()
        .clamp(0.0, surface_height) as u32;
    let max_x = (logical_rect.max.x * pixels_per_point)
        .ceil()
        .clamp(0.0, surface_width) as u32;
    let max_y = (logical_rect.max.y * pixels_per_point)
        .ceil()
        .clamp(0.0, surface_height) as u32;
    let width = max_x.saturating_sub(min_x);
    let height = max_y.saturating_sub(min_y);

    PlotRectPx::try_new(
        min_x,
        min_y,
        width,
        height,
        surface_width_px,
        surface_height_px,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use egui::{pos2, vec2, Context, Panel, RawInput, Rect};

    use super::*;

    #[test]
    fn ui_plot_surface_reports_central_region_after_panels() {
        let cases = [
            ("normal", 1_200.0, 800.0, 1.0, 58.0, 380.0),
            ("narrow-collapsed", 640.0, 480.0, 1.0, 42.0, 42.0),
            ("hidpi", 1_600.0, 900.0, 1.5, 58.0, 380.0),
        ];

        for (name, width, height, pixels_per_point, left_width, right_width) in cases {
            let context = Context::default();
            let physical_width = (width * pixels_per_point) as u32;
            let physical_height = (height * pixels_per_point) as u32;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))),
                ..RawInput::default()
            };
            let mut layout = None;

            let _ = context.run_ui(input, |ui| {
                Panel::top("test_top").exact_size(50.0).show(ui, |_ui| {});
                Panel::left("test_left")
                    .exact_size(left_width)
                    .show(ui, |_ui| {});
                Panel::right("test_right")
                    .exact_size(right_width)
                    .show(ui, |_ui| {});
                Panel::bottom("test_bottom")
                    .exact_size(32.0)
                    .show(ui, |_ui| {});
                layout =
                    allocate_plot_surface(ui, pixels_per_point, physical_width, physical_height);
            });

            let layout = layout.expect("remaining central plot should have area");
            assert!(
                layout.physical_rect.width > 0 && layout.physical_rect.height > 0,
                "{name} shell must retain a physical plot"
            );
            assert!(layout.physical_rect.x + layout.physical_rect.width <= physical_width);
            assert!(layout.physical_rect.y + layout.physical_rect.height <= physical_height);
            assert_eq!(layout.logical_rect, layout.axis_layout.plot_rect);
            assert!(layout
                .axis_layout
                .outer_rect
                .contains_rect(layout.logical_rect));
        }
    }

    #[test]
    fn logical_plot_conversion_rounds_outward_and_clamps_to_surface() {
        let logical_rect = Rect::from_min_max(pos2(-2.0, 10.2), pos2(700.4, 500.2));

        let physical = logical_to_physical_plot_rect(logical_rect, 1.5, 1_000, 700)
            .expect("clamped plot should retain area");

        assert_eq!(
            physical,
            PlotRectPx::try_new(0, 15, 1_000, 685, 1_000, 700).unwrap()
        );
    }

    #[test]
    fn marginal_gutters_do_not_change_plot_hit_rect() {
        let outer_rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(700.0, 500.0));
        let layout = PlotAxisLayout::from_outer_rect(outer_rect).unwrap();

        assert_eq!(layout.plot_rect.left(), PLOT_LEFT_GUTTER_POINTS);
        assert_eq!(layout.plot_rect.top(), PLOT_TOP_GUTTER_POINTS);
        assert!(layout.plot_rect.right() < layout.outer_rect.right());
        assert!(layout.plot_rect.bottom() < layout.outer_rect.bottom());
    }
}
