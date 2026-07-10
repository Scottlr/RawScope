//! Difference-density mode controls and truthful diverging legend.

use egui::{Align, Color32, Layout, Rect, Sense, Ui};
use rawscope_render::{DifferencePalette, ScatterDensityMode, ScatterDifferenceRenderStats};

use crate::ui_theme::segmented_button;

const DIFFERENCE_LEGEND_HEIGHT_PX: f32 = 16.0;

pub(crate) fn density_mode_selector(
    ui: &mut Ui,
    active: ScatterDensityMode,
    difference_available: bool,
) -> Option<ScatterDensityMode> {
    let mut selected = None;
    ui.horizontal(|ui| {
        ui.label("Mode");
        for mode in [
            ScatterDensityMode::AbsoluteDensity,
            ScatterDensityMode::FilteredDifference,
        ] {
            let enabled = mode == ScatterDensityMode::AbsoluteDensity || difference_available;
            if ui
                .add_enabled(enabled, segmented_button(mode.label(), mode == active))
                .clicked()
                && mode != active
            {
                selected = Some(mode);
            }
        }
    });
    selected
}

pub(crate) fn draw_difference_legend(ui: &mut Ui, stats: Option<ScatterDifferenceRenderStats>) {
    ui.label("Active share minus full-baseline share");
    let colors = DifferencePalette::TealNeutralCoral.legend_rgb();
    let width = ui.available_width().max(120.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width, DIFFERENCE_LEGEND_HEIGHT_PX),
        Sense::hover(),
    );
    let segment_width = rect.width() / colors.len() as f32;
    for (index, [red, green, blue]) in colors.into_iter().enumerate() {
        let min_x = rect.left() + segment_width * index as f32;
        let segment = Rect::from_min_max(
            egui::pos2(min_x, rect.top()),
            egui::pos2((min_x + segment_width).min(rect.right()), rect.bottom()),
        );
        ui.painter()
            .rect_filled(segment, 2.0, Color32::from_rgb(red, green, blue));
    }
    ui.horizontal(|ui| {
        ui.label("under-represented");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label("over-represented")
        });
    });
    if let Some(stats) = stats {
        ui.label(format!(
            "Active {} | baseline {} rows",
            stats.active_total, stats.baseline_total
        ));
    }
}
