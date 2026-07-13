//! Stable, edge-aware inspection tooltip placement, motion, and painting.

use std::time::Duration;

use egui::{pos2, vec2, Area, Color32, Context, Frame, Id, Order, Rect, RichText, Vec2};
use rawscope_render::{format_axis_value, AxisValueFormat, DifferenceDirection};

use crate::ui_scatter_inspection::ScatterInspectionUiState;

pub(crate) const TOOLTIP_GAP_PX: f32 = 10.0;
pub(crate) const TOOLTIP_VIEWPORT_MARGIN_PX: f32 = 8.0;
pub(crate) const TOOLTIP_ENTER_DURATION_MS: u64 = 110;
pub(crate) const TOOLTIP_EXIT_DURATION_MS: u64 = 80;
pub(crate) const TOOLTIP_ENTER_TRANSLATE_Y_PX: f32 = 4.0;
pub(crate) const TOOLTIP_REPAINT_INTERVAL_MS: u64 = 16;

const TOOLTIP_WIDTH_PX: f32 = 280.0;
const TOOLTIP_HEIGHT_PX: f32 = 176.0;
const TOOLTIP_BODY_COLOR: Color32 = Color32::from_rgb(205, 216, 216);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct InspectionPresentationKey {
    pub(crate) viewport_revision: u64,
    pub(crate) filter_revision: rawscope_data::FilterRevision,
    pub(crate) bin_x: u32,
    pub(crate) bin_y: u32,
    pub(crate) density_mode: rawscope_render::ScatterDensityMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TooltipPlacementInput {
    pub(crate) bin_rect: Rect,
    pub(crate) tooltip_size: Vec2,
    pub(crate) viewport_rect: Rect,
    pub(crate) gap_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TooltipSide {
    Right,
    Left,
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TooltipPlacement {
    pub(crate) rect: Rect,
    pub(crate) side: TooltipSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct InspectionPresentationFrame {
    pub(crate) opacity: f32,
    pub(crate) translate_y_px: f32,
    pub(crate) running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionPhase {
    Hidden,
    Entering,
    Visible,
    Exiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InspectionPresentationMotion {
    phase: MotionPhase,
    key: Option<InspectionPresentationKey>,
    started_at_ms: u64,
}

impl Default for InspectionPresentationMotion {
    fn default() -> Self {
        Self {
            phase: MotionPhase::Hidden,
            key: None,
            started_at_ms: 0,
        }
    }
}

impl InspectionPresentationMotion {
    pub(crate) fn sync(
        &mut self,
        key: Option<InspectionPresentationKey>,
        now_ms: u64,
        reduced_motion: bool,
    ) {
        if reduced_motion {
            self.key = key;
            self.phase = if key.is_some() {
                MotionPhase::Visible
            } else {
                MotionPhase::Hidden
            };
            self.started_at_ms = now_ms;
            return;
        }
        if self.key == key {
            return;
        }
        self.key = key;
        self.started_at_ms = now_ms;
        self.phase = if key.is_some() {
            MotionPhase::Entering
        } else if self.phase == MotionPhase::Hidden {
            MotionPhase::Hidden
        } else {
            MotionPhase::Exiting
        };
    }

    pub(crate) fn frame(
        &mut self,
        now_ms: u64,
        reduced_motion: bool,
    ) -> InspectionPresentationFrame {
        if reduced_motion {
            return InspectionPresentationFrame {
                opacity: if self.key.is_some() { 1.0 } else { 0.0 },
                translate_y_px: 0.0,
                running: false,
            };
        }
        let elapsed_ms = now_ms.saturating_sub(self.started_at_ms);
        match self.phase {
            MotionPhase::Hidden => InspectionPresentationFrame {
                opacity: 0.0,
                translate_y_px: 0.0,
                running: false,
            },
            MotionPhase::Visible => InspectionPresentationFrame {
                opacity: 1.0,
                translate_y_px: 0.0,
                running: false,
            },
            MotionPhase::Entering => {
                if elapsed_ms >= TOOLTIP_ENTER_DURATION_MS {
                    self.phase = MotionPhase::Visible;
                    return InspectionPresentationFrame {
                        opacity: 1.0,
                        translate_y_px: 0.0,
                        running: false,
                    };
                }
                let progress = elapsed_ms as f32 / TOOLTIP_ENTER_DURATION_MS as f32;
                let eased = ease_out_cubic(progress);
                InspectionPresentationFrame {
                    opacity: eased,
                    translate_y_px: (1.0 - eased) * TOOLTIP_ENTER_TRANSLATE_Y_PX,
                    running: true,
                }
            }
            MotionPhase::Exiting => {
                if elapsed_ms >= TOOLTIP_EXIT_DURATION_MS {
                    self.phase = MotionPhase::Hidden;
                    return InspectionPresentationFrame {
                        opacity: 0.0,
                        translate_y_px: 0.0,
                        running: false,
                    };
                }
                let progress = elapsed_ms as f32 / TOOLTIP_EXIT_DURATION_MS as f32;
                InspectionPresentationFrame {
                    opacity: 1.0 - ease_out_cubic(progress),
                    translate_y_px: 0.0,
                    running: true,
                }
            }
        }
    }

    pub(crate) fn is_hidden(self) -> bool {
        self.phase == MotionPhase::Hidden
    }
}

pub(crate) fn place_tooltip(input: TooltipPlacementInput) -> TooltipPlacement {
    let candidates = [
        (
            TooltipSide::Right,
            pos2(input.bin_rect.right() + input.gap_px, input.bin_rect.top()),
        ),
        (
            TooltipSide::Left,
            pos2(
                input.bin_rect.left() - input.gap_px - input.tooltip_size.x,
                input.bin_rect.top(),
            ),
        ),
        (
            TooltipSide::Above,
            pos2(
                input.bin_rect.left(),
                input.bin_rect.top() - input.gap_px - input.tooltip_size.y,
            ),
        ),
        (
            TooltipSide::Below,
            pos2(
                input.bin_rect.left(),
                input.bin_rect.bottom() + input.gap_px,
            ),
        ),
    ];
    let bounds = input.viewport_rect.shrink(TOOLTIP_VIEWPORT_MARGIN_PX);
    for (side, origin) in candidates.iter().copied() {
        let rect = Rect::from_min_size(origin, input.tooltip_size);
        if bounds.contains_rect(rect) && !rect.intersects(input.bin_rect) {
            return TooltipPlacement { rect, side };
        }
    }

    let clamped_candidates = candidates.map(|(side, origin)| {
        (
            side,
            clamp_rect(Rect::from_min_size(origin, input.tooltip_size), bounds),
        )
    });
    if let Some((side, rect)) = clamped_candidates
        .into_iter()
        .find(|(_, rect)| !rect.intersects(input.bin_rect))
    {
        return TooltipPlacement { rect, side };
    }

    let (side, candidate) = candidates
        .into_iter()
        .max_by(|(_, left), (_, right)| {
            visible_area(
                Rect::from_min_size(*left, input.tooltip_size),
                input.viewport_rect,
            )
            .total_cmp(&visible_area(
                Rect::from_min_size(*right, input.tooltip_size),
                input.viewport_rect,
            ))
        })
        .expect("tooltip placement has four candidates");
    TooltipPlacement {
        rect: clamp_rect(Rect::from_min_size(candidate, input.tooltip_size), bounds),
        side,
    }
}

fn visible_area(rect: Rect, viewport: Rect) -> f32 {
    rect.intersect(viewport).area()
}

fn clamp_rect(rect: Rect, bounds: Rect) -> Rect {
    let max_x = (bounds.right() - rect.width()).max(bounds.left());
    let max_y = (bounds.bottom() - rect.height()).max(bounds.top());
    Rect::from_min_size(
        pos2(
            rect.left().clamp(bounds.left(), max_x),
            rect.top().clamp(bounds.top(), max_y),
        ),
        rect.size(),
    )
}

fn ease_out_cubic(progress: f32) -> f32 {
    let remaining = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - remaining * remaining * remaining
}

pub(crate) fn show_inspection_tooltip(
    context: &Context,
    content: Option<&ScatterInspectionUiState>,
    frame: InspectionPresentationFrame,
) {
    let Some(content) = content else { return };
    let Some(bin_rect) = content.hovered_bin_rect else {
        return;
    };
    if frame.opacity <= 0.0 {
        return;
    }
    let placement = place_tooltip(TooltipPlacementInput {
        bin_rect,
        tooltip_size: vec2(TOOLTIP_WIDTH_PX, TOOLTIP_HEIGHT_PX),
        viewport_rect: context.viewport_rect(),
        gap_px: TOOLTIP_GAP_PX,
    });
    Area::new(Id::new("scatter_inspection_tooltip"))
        .order(Order::Tooltip)
        .fixed_pos(placement.rect.min + vec2(0.0, frame.translate_y_px))
        .interactable(false)
        .show(context, |ui| {
            ui.set_opacity(frame.opacity);
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(TOOLTIP_WIDTH_PX - 24.0);
                ui.set_max_width(TOOLTIP_WIDTH_PX - 24.0);
                show_tooltip_content(ui, content);
            });
        });
    if frame.running {
        context.request_repaint_after(Duration::from_millis(TOOLTIP_REPAINT_INTERVAL_MS));
    }
}

fn show_tooltip_content(ui: &mut egui::Ui, content: &ScatterInspectionUiState) {
    let hit = content
        .hovered
        .as_ref()
        .expect("presentation content always retains its inspected hit");
    ui.label(
        RichText::new(format!(
            "{} {} .. {}",
            content.x_label,
            format_axis_value(hit.x_range.min, AxisValueFormat::Compact),
            format_axis_value(hit.x_range.max, AxisValueFormat::Compact),
        ))
        .small()
        .color(TOOLTIP_BODY_COLOR),
    );
    ui.label(
        RichText::new(format!(
            "{} {} .. {}",
            content.y_label,
            format_axis_value(hit.y_range.min, AxisValueFormat::Compact),
            format_axis_value(hit.y_range.max, AxisValueFormat::Compact),
        ))
        .small()
        .color(TOOLTIP_BODY_COLOR),
    );
    ui.add_space(3.0);
    ui.label(RichText::new(format!("{} rows", hit.count)).strong());
    if let Some(summary) = content.hovered_summary.as_ref() {
        ui.label(format!(
            "{:.2}% of active rows",
            summary.active_share * 100.0
        ));
        if let Some(percentile) = summary.occupied_density_percentile {
            ui.label(format!(
                "{:.1}th percentile among occupied cells",
                percentile * 100.0
            ));
        }
        ui.label(format!(
            "Nearby 3x3: {} rows ({:.2}% of active)",
            summary.neighborhood_count,
            summary.neighborhood_share * 100.0
        ));
    }
    if let Some(difference) = content.hovered_difference {
        ui.separator();
        ui.label(format!(
            "Active {:.3}% | full baseline {:.3}%",
            difference.active_share * 100.0,
            difference.baseline_share * 100.0
        ));
        ui.label(format!(
            "Counts: {} active / {} baseline ({} / {} total)",
            difference.active_count,
            difference.baseline_count,
            difference.active_total,
            difference.baseline_total
        ));
        ui.label(format!(
            "Support context: {:.3}% (presentation aid, not significance)",
            difference.support_share * 100.0
        ));
        let direction = match difference.direction {
            DifferenceDirection::MoreCommonInActive => "More common in active",
            DifferenceDirection::LessCommonInActive => "Less common in active",
            DifferenceDirection::Unchanged => "Unchanged",
        };
        ui.label(format!(
            "{direction}: {:+.3} percentage points",
            difference.share_delta * 100.0
        ));
        if let Some(percentile) = difference.absolute_delta_percentile {
            ui.label(format!(
                "Difference strength: {:.1}th percentile",
                percentile * 100.0
            ));
        }
    }
    if !hit.row_ids.is_empty() {
        let sample_count = hit.row_ids.len();
        ui.label(
            RichText::new(if sample_count < hit.count as usize {
                format!("{sample_count} sampled rows of {} in bin", hit.count)
            } else {
                format!("{sample_count} rows in bin")
            })
            .small()
            .color(Color32::from_rgb(145, 158, 158)),
        );
    }
}

#[cfg(test)]
#[path = "ui_inspection_tooltip_tests.rs"]
mod tests;
