//! Visual encoding context for density surfaces.

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Ui};
use rawscope_render::{
    DensityEncoding, DensityPalette, DensityTransform, ScatterDensityPresentation,
};

use crate::{app::WorkbenchApp, demo::DemoMode, ui::WorkbenchSurface, ui_theme::segmented_button};

const DENSITY_LEGEND_HEIGHT_PX: f32 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DensityEncodingUiState {
    pub(crate) encoding: DensityEncoding,
    pub(crate) scatter_presentation: Option<ScatterDensityPresentation>,
    pub(crate) surface_label: String,
    pub(crate) measure_label: String,
    pub(crate) bin_grid_label: String,
    pub(crate) scale_label: String,
    pub(crate) palette_label: String,
    pub(crate) range_label: String,
    pub(crate) zero_label: String,
    pub(crate) max_label: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DensityEncodingResponse {
    pub(crate) shown: bool,
    pub(crate) set_transform: Option<DensityTransform>,
    pub(crate) set_scatter_presentation: Option<ScatterDensityPresentation>,
}

impl DensityEncodingUiState {
    fn scatter(
        encoding: DensityEncoding,
        presentation: ScatterDensityPresentation,
        grid_width: u32,
        grid_height: u32,
        max_bin_count: u32,
        max_bin_count_is_current: bool,
    ) -> Self {
        Self::new(
            encoding,
            Some(presentation),
            "Scatter density",
            "Rows per bin",
            grid_width,
            grid_height,
            "rows/bin",
            max_bin_count,
            max_bin_count_is_current,
        )
    }

    fn timeline(
        encoding: DensityEncoding,
        grid_width: u32,
        grid_height: u32,
        max_bin_count: u32,
    ) -> Self {
        Self::new(
            encoding,
            None,
            "Timeline density",
            "Events per bin",
            grid_width,
            grid_height,
            "events/bin",
            max_bin_count,
            true,
        )
    }

    fn new(
        encoding: DensityEncoding,
        scatter_presentation: Option<ScatterDensityPresentation>,
        surface_label: &str,
        measure_label: &str,
        grid_width: u32,
        grid_height: u32,
        count_unit: &str,
        max_bin_count: u32,
        max_bin_count_is_current: bool,
    ) -> Self {
        Self {
            encoding,
            scatter_presentation,
            surface_label: surface_label.to_string(),
            measure_label: measure_label.to_string(),
            bin_grid_label: format!("Grid {grid_width}x{grid_height} bins"),
            scale_label: format!(
                "Scale {}, normalized to {}",
                encoding.transform.label(),
                encoding.normalization.label()
            ),
            palette_label: format!("Palette {}", encoding.palette.label()),
            range_label: format!(
                "Range 0..{max_bin_count} {count_unit}{}",
                if max_bin_count_is_current {
                    ""
                } else {
                    " (last readback)"
                }
            ),
            zero_label: "0".to_string(),
            max_label: format!("max {max_bin_count}"),
        }
    }
}

pub(crate) fn density_encoding_ui_state(app: &WorkbenchApp) -> Option<DensityEncodingUiState> {
    if app.visible_surface != WorkbenchSurface::Primary {
        return None;
    }

    match app.demo_mode {
        DemoMode::Scatter => {
            let stats = app.scatter.render_stats?;
            Some(DensityEncodingUiState::scatter(
                app.scatter.density_encoding,
                app.scatter.density_presentation,
                stats.grid_width,
                stats.grid_height,
                stats.max_bin_count,
                stats.max_bin_count_is_current,
            ))
        }
        DemoMode::Timeline => {
            let stats = app.timeline.render_stats?;
            Some(DensityEncodingUiState::timeline(
                app.timeline.density_encoding,
                stats.grid_width,
                stats.grid_height,
                stats.max_bin_count,
            ))
        }
    }
}

pub(crate) fn show_density_encoding(
    ui: &mut Ui,
    encoding: Option<&DensityEncodingUiState>,
) -> DensityEncodingResponse {
    let Some(encoding) = encoding else {
        return DensityEncodingResponse::default();
    };

    ui.heading("Visual Encoding");
    ui.label(RichText::new(&encoding.surface_label).strong());
    ui.label(&encoding.measure_label);
    ui.label(&encoding.bin_grid_label);
    ui.label(&encoding.scale_label);
    ui.label(&encoding.palette_label);
    ui.label(&encoding.range_label);
    let set_scatter_presentation = encoding
        .scatter_presentation
        .and_then(|presentation| presentation_selector(ui, presentation));
    let set_transform = transform_selector(ui, encoding.encoding.transform);
    draw_density_legend(ui, encoding);

    DensityEncodingResponse {
        shown: true,
        set_transform,
        set_scatter_presentation,
    }
}

fn presentation_selector(
    ui: &mut Ui,
    active_presentation: ScatterDensityPresentation,
) -> Option<ScatterDensityPresentation> {
    let mut selected_presentation = None;

    ui.horizontal(|ui| {
        ui.label("Surface");
        for presentation in [
            ScatterDensityPresentation::ExactCells,
            ScatterDensityPresentation::TopographicField,
        ] {
            let response = ui
                .add(segmented_button(
                    presentation.label(),
                    active_presentation == presentation,
                ))
                .on_hover_text(match presentation {
                    ScatterDensityPresentation::ExactCells => {
                        "Show the exact GPU density bins without interpolation."
                    }
                    ScatterDensityPresentation::TopographicField => {
                        "Reconstruct a smooth density field with contours and gradient relief."
                    }
                });
            if response.clicked() && active_presentation != presentation {
                selected_presentation = Some(presentation);
            }
        }
    });

    selected_presentation
}

fn transform_selector(ui: &mut Ui, active_transform: DensityTransform) -> Option<DensityTransform> {
    let mut selected_transform = None;

    ui.horizontal(|ui| {
        ui.label("Transform");
        for transform in [DensityTransform::Linear, DensityTransform::Log1p] {
            let response = ui.add(segmented_button(
                transform.label(),
                active_transform == transform,
            ));
            if response.clicked() && active_transform != transform {
                selected_transform = Some(transform);
            }
        }
    });

    selected_transform
}

fn draw_density_legend(ui: &mut Ui, encoding: &DensityEncodingUiState) {
    let legend_width_px = ui.available_width().max(120.0);
    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(legend_width_px, DENSITY_LEGEND_HEIGHT_PX),
        Sense::hover(),
    );
    let colors = density_palette_colors(encoding.encoding.palette);
    let segment_width_px = rect.width() / colors.len() as f32;

    for (segment_index, color) in colors.iter().enumerate() {
        let segment_min_x = rect.left() + segment_width_px * segment_index as f32;
        let segment_max_x = if segment_index + 1 == colors.len() {
            rect.right()
        } else {
            segment_min_x + segment_width_px
        };
        let segment_rect = Rect::from_min_max(
            egui::pos2(segment_min_x, rect.top()),
            egui::pos2(segment_max_x, rect.bottom()),
        );
        ui.painter().rect_filled(segment_rect, 2.0, *color);
    }

    ui.horizontal(|ui| {
        ui.label(&encoding.zero_label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(&encoding.max_label);
        });
    });
}

fn density_palette_colors(palette: DensityPalette) -> [Color32; 4] {
    palette
        .legend_rgb()
        .map(|[red, green, blue]| Color32::from_rgb(red, green, blue))
}

#[cfg(test)]
mod tests {
    use rawscope_render::{
        DensityEncoding, DensityPalette, DensityTransform, ScatterDensityPresentation,
    };

    use super::DensityEncodingUiState;

    #[test]
    fn scatter_encoding_makes_density_transform_and_range_visible() {
        let encoding = DensityEncodingUiState::scatter(
            DensityEncoding::scatter_default(),
            ScatterDensityPresentation::TopographicField,
            256,
            256,
            42,
            true,
        );

        assert_eq!(encoding.surface_label, "Scatter density");
        assert_eq!(encoding.bin_grid_label, "Grid 256x256 bins");
        assert_eq!(
            encoding.scale_label,
            "Scale log1p(count), normalized to viewport max"
        );
        assert_eq!(encoding.palette_label, "Palette scatter sequential");
        assert_eq!(encoding.range_label, "Range 0..42 rows/bin");
        assert_eq!(encoding.max_label, "max 42");
        assert_eq!(
            encoding.scatter_presentation,
            Some(ScatterDensityPresentation::TopographicField)
        );
        assert_eq!(encoding.encoding.palette, DensityPalette::ScatterSequential);
    }

    #[test]
    fn timeline_encoding_uses_event_units() {
        let encoding =
            DensityEncodingUiState::timeline(DensityEncoding::timeline_default(), 256, 12, 7);

        assert_eq!(encoding.surface_label, "Timeline density");
        assert_eq!(encoding.measure_label, "Events per bin");
        assert_eq!(encoding.bin_grid_label, "Grid 256x12 bins");
        assert_eq!(encoding.range_label, "Range 0..7 events/bin");
        assert_eq!(
            encoding.encoding.palette,
            DensityPalette::TimelineSequential
        );
    }

    #[test]
    fn encoding_state_reflects_active_transform() {
        let active_encoding =
            DensityEncoding::scatter_default().with_transform(DensityTransform::Linear);
        let encoding = DensityEncodingUiState::scatter(
            active_encoding,
            ScatterDensityPresentation::ExactCells,
            128,
            128,
            8,
            true,
        );

        assert_eq!(encoding.encoding.transform, DensityTransform::Linear);
        assert_eq!(
            encoding.scale_label,
            "Scale linear(count), normalized to viewport max"
        );
    }
}
