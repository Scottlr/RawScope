//! Non-interactive axis, grid, and reference-guide painting for the plot.

use egui::{Align2, Color32, FontId, Id, LayerId, Order, Painter, Pos2, Stroke, Ui};

use crate::{ui::WorkbenchViewAxes, ui_plot_surface::PlotAxisLayout};

const AXIS_FONT_SIZE_POINTS: f32 = 11.0;
const TICK_LENGTH_POINTS: f32 = 4.0;
const TICK_TEXT_GAP_POINTS: f32 = 5.0;
const AXIS_LABEL_GAP_POINTS: f32 = 22.0;
const GUIDE_LABEL_INSET_POINTS: f32 = 8.0;
const GUIDE_LABEL_VERTICAL_OFFSET_POINTS: f32 = 6.0;

pub(crate) fn show_plot_axes(
    ui: &mut Ui,
    layout: PlotAxisLayout,
    axes: Option<&WorkbenchViewAxes>,
) {
    let Some(axes) = axes else { return };
    let painter = ui.ctx().layer_painter(LayerId::new(
        Order::Foreground,
        Id::new("rawscope_plot_axes"),
    ));
    let plot_painter = painter.with_clip_rect(layout.plot_rect);
    let font = FontId::monospace(AXIS_FONT_SIZE_POINTS);
    let grid_stroke = Stroke::new(1.0, Color32::from_gray(64));
    let guide_stroke = Stroke::new(1.5, Color32::from_rgb(214, 174, 72));
    let tick_stroke = Stroke::new(1.0, Color32::from_gray(144));
    let text_color = Color32::from_gray(218);

    match axes {
        WorkbenchViewAxes::Scatter(context) => {
            draw_x_axis(
                &painter,
                &plot_painter,
                layout,
                &context.x,
                &font,
                text_color,
                tick_stroke,
                grid_stroke,
            );
            draw_y_axis(
                &painter,
                &plot_painter,
                layout,
                &context.y,
                &font,
                text_color,
                tick_stroke,
                grid_stroke,
            );
            for guide in &context.guides {
                let start = fraction_point(layout.plot_rect, guide.start_fraction);
                let end = fraction_point(layout.plot_rect, guide.end_fraction);
                plot_painter.line_segment([start, end], guide_stroke);
                let (label_position, label_alignment) = guide_label_layout(start, end, guide.kind);
                painter.text(
                    label_position,
                    label_alignment,
                    &guide.label,
                    font.clone(),
                    Color32::from_rgb(226, 193, 105),
                );
            }
        }
        WorkbenchViewAxes::Timeline(context) => {
            draw_x_axis(
                &painter,
                &plot_painter,
                layout,
                &context.time,
                &font,
                text_color,
                tick_stroke,
                grid_stroke,
            );
            for lane in &context.lanes {
                let y = layout.plot_rect.top() + lane.fraction * layout.plot_rect.height();
                plot_painter.line_segment(
                    [
                        Pos2::new(layout.plot_rect.left(), y),
                        Pos2::new(layout.plot_rect.right(), y),
                    ],
                    grid_stroke,
                );
                painter.text(
                    Pos2::new(layout.plot_rect.left() - TICK_TEXT_GAP_POINTS, y),
                    Align2::RIGHT_CENTER,
                    &lane.label,
                    font.clone(),
                    text_color,
                );
            }
        }
    }
}

fn draw_x_axis(
    painter: &Painter,
    plot_painter: &Painter,
    layout: PlotAxisLayout,
    axis: &rawscope_render::NumericAxisContext,
    font: &FontId,
    text_color: Color32,
    tick_stroke: Stroke,
    grid_stroke: Stroke,
) {
    for (tick_index, tick) in axis.ticks.iter().enumerate() {
        let x = layout.plot_rect.left() + tick.fraction * layout.plot_rect.width();
        plot_painter.line_segment(
            [
                Pos2::new(x, layout.plot_rect.top()),
                Pos2::new(x, layout.plot_rect.bottom()),
            ],
            grid_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(x, layout.plot_rect.bottom()),
                Pos2::new(x, layout.plot_rect.bottom() + TICK_LENGTH_POINTS),
            ],
            tick_stroke,
        );
        if tick_label_is_visible(tick_index, axis.ticks.len(), layout.plot_rect.width(), 72.0) {
            painter.text(
                Pos2::new(x, layout.plot_rect.bottom() + TICK_TEXT_GAP_POINTS),
                Align2::CENTER_TOP,
                &tick.label,
                font.clone(),
                text_color,
            );
        }
    }
    painter.text(
        Pos2::new(
            layout.plot_rect.center().x,
            layout.plot_rect.bottom() + AXIS_LABEL_GAP_POINTS,
        ),
        Align2::CENTER_TOP,
        &axis.label,
        font.clone(),
        text_color,
    );
}

fn draw_y_axis(
    painter: &Painter,
    plot_painter: &Painter,
    layout: PlotAxisLayout,
    axis: &rawscope_render::NumericAxisContext,
    font: &FontId,
    text_color: Color32,
    tick_stroke: Stroke,
    grid_stroke: Stroke,
) {
    for (tick_index, tick) in axis.ticks.iter().enumerate() {
        let y = layout.plot_rect.bottom() - tick.fraction * layout.plot_rect.height();
        plot_painter.line_segment(
            [
                Pos2::new(layout.plot_rect.left(), y),
                Pos2::new(layout.plot_rect.right(), y),
            ],
            grid_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(layout.plot_rect.left() - TICK_LENGTH_POINTS, y),
                Pos2::new(layout.plot_rect.left(), y),
            ],
            tick_stroke,
        );
        if tick_label_is_visible(
            tick_index,
            axis.ticks.len(),
            layout.plot_rect.height(),
            28.0,
        ) {
            painter.text(
                Pos2::new(layout.plot_rect.left() - TICK_TEXT_GAP_POINTS, y),
                Align2::RIGHT_CENTER,
                &tick.label,
                font.clone(),
                text_color,
            );
        }
    }
    painter.text(
        Pos2::new(layout.outer_rect.left() + 4.0, layout.plot_rect.center().y),
        Align2::LEFT_CENTER,
        &axis.label,
        font.clone(),
        text_color,
    );
}

fn fraction_point(rect: egui::Rect, fraction: (f32, f32)) -> Pos2 {
    Pos2::new(
        rect.left() + fraction.0 * rect.width(),
        rect.top() + fraction.1 * rect.height(),
    )
}

fn tick_label_is_visible(
    index: usize,
    tick_count: usize,
    available: f32,
    min_spacing: f32,
) -> bool {
    if tick_count <= 2 {
        return true;
    }
    let max_labels = ((available / min_spacing).floor() as usize).max(2);
    if tick_count <= max_labels {
        return true;
    }
    let intervals = tick_count - 1;
    let visible_intervals = max_labels - 1;
    let stride = intervals.div_ceil(visible_intervals);
    index == 0 || index + 1 == tick_count || index.is_multiple_of(stride)
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};

    use super::*;

    #[test]
    fn plot_axes_projection_uses_authoritative_layout() {
        let layout = PlotAxisLayout {
            outer_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(700.0, 500.0)),
            plot_rect: Rect::from_min_max(pos2(58.0, 10.0), pos2(688.0, 462.0)),
        };

        assert_eq!(
            fraction_point(layout.plot_rect, (0.0, 0.0)),
            layout.plot_rect.min
        );
        assert_eq!(
            fraction_point(layout.plot_rect, (1.0, 1.0)),
            layout.plot_rect.max
        );
        assert_eq!(
            fraction_point(layout.plot_rect, (0.5, 0.5)),
            layout.plot_rect.center()
        );
    }

    #[test]
    fn narrow_plot_reduces_visible_tick_labels() {
        let visible = (0..9)
            .filter(|index| tick_label_is_visible(*index, 9, 180.0, 72.0))
            .count();

        assert!(visible <= 3);
        assert!(tick_label_is_visible(0, 9, 180.0, 72.0));
        assert!(tick_label_is_visible(8, 9, 180.0, 72.0));
    }

    #[test]
    fn guide_label_is_inset_from_y_axis() {
        let plot_rect = Rect::from_min_max(pos2(58.0, 10.0), pos2(688.0, 462.0));
        let guide_start = fraction_point(plot_rect, (0.0, 0.5));
        let guide_end = fraction_point(plot_rect, (1.0, 0.5));
        let (label_position, alignment) = guide_label_layout(
            guide_start,
            guide_end,
            rawscope_render::ScatterReferenceGuideKind::Horizontal { y: 0.0 },
        );

        assert!(label_position.x > plot_rect.center().x);
        assert_eq!(alignment, Align2::RIGHT_BOTTOM);
        assert!(label_position.y < plot_rect.center().y);
    }
}

fn guide_label_layout(
    start: Pos2,
    end: Pos2,
    kind: rawscope_render::ScatterReferenceGuideKind,
) -> (Pos2, Align2) {
    match kind {
        rawscope_render::ScatterReferenceGuideKind::Horizontal { .. } => (
            Pos2::new(
                end.x - GUIDE_LABEL_INSET_POINTS,
                end.y - GUIDE_LABEL_VERTICAL_OFFSET_POINTS,
            ),
            Align2::RIGHT_BOTTOM,
        ),
        _ => (
            Pos2::new(
                start.x + GUIDE_LABEL_INSET_POINTS,
                start.y - GUIDE_LABEL_VERTICAL_OFFSET_POINTS,
            ),
            Align2::LEFT_BOTTOM,
        ),
    }
}
