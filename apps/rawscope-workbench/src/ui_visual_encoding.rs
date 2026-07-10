//! Visual encoding context for density surfaces.

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Ui};
use rawscope_data::ScatterProjection;
use rawscope_render::{
    DensityEncoding, DensityPalette, DensityTransform, PointRevealMode, PointRevealStats,
    ReliefFieldConfig, ScatterDensityMode, ScatterDensityPresentation,
    ScatterDifferenceRenderStats,
};

use crate::{
    app::WorkbenchApp,
    demo::DemoMode,
    ui::WorkbenchSurface,
    ui_difference_density::{density_mode_selector, draw_difference_legend},
    ui_relief::show_relief_controls,
    ui_theme::segmented_button,
};

const DENSITY_LEGEND_HEIGHT_PX: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
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
    pub(crate) point_reveal_mode: Option<PointRevealMode>,
    pub(crate) point_reveal_stats: Option<PointRevealStats>,
    pub(crate) scatter_projection: Option<ScatterProjection>,
    pub(crate) scatter_density_mode: Option<ScatterDensityMode>,
    pub(crate) difference_available: bool,
    pub(crate) difference_stats: Option<ScatterDifferenceRenderStats>,
    pub(crate) relief_config: Option<ReliefFieldConfig>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct DensityEncodingResponse {
    pub(crate) shown: bool,
    pub(crate) set_transform: Option<DensityTransform>,
    pub(crate) set_scatter_presentation: Option<ScatterDensityPresentation>,
    pub(crate) set_point_reveal_mode: Option<PointRevealMode>,
    pub(crate) set_scatter_projection: Option<ScatterProjection>,
    pub(crate) set_scatter_density_mode: Option<ScatterDensityMode>,
    pub(crate) set_relief_config: Option<ReliefFieldConfig>,
}

impl DensityEncodingUiState {
    fn scatter(
        encoding: DensityEncoding,
        presentation: ScatterDensityPresentation,
        grid_width: u32,
        grid_height: u32,
        max_bin_count: u32,
        max_bin_count_is_current: bool,
        point_reveal_mode: PointRevealMode,
        point_reveal_stats: Option<PointRevealStats>,
        scatter_projection: Option<ScatterProjection>,
        scatter_density_mode: ScatterDensityMode,
        difference_available: bool,
        difference_stats: Option<ScatterDifferenceRenderStats>,
        relief_config: ReliefFieldConfig,
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
            Some(point_reveal_mode),
            point_reveal_stats,
            scatter_projection,
            Some(scatter_density_mode),
            difference_available,
            difference_stats,
            Some(relief_config),
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
            None,
            None,
            None,
            None,
            false,
            None,
            None,
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
        point_reveal_mode: Option<PointRevealMode>,
        point_reveal_stats: Option<PointRevealStats>,
        scatter_projection: Option<ScatterProjection>,
        scatter_density_mode: Option<ScatterDensityMode>,
        difference_available: bool,
        difference_stats: Option<ScatterDifferenceRenderStats>,
        relief_config: Option<ReliefFieldConfig>,
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
            point_reveal_mode,
            point_reveal_stats,
            scatter_projection,
            scatter_density_mode,
            difference_available,
            difference_stats,
            relief_config,
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
                app.point_reveal.config.mode,
                app.point_reveal.stats,
                app.scatter_projection
                    .available
                    .then_some(app.scatter_projection.active),
                app.scatter.density_mode,
                app.scatter_filters.is_active()
                    && app
                        .scatter_filters
                        .evaluation
                        .as_ref()
                        .is_some_and(|evaluation| evaluation.included_count > 0),
                app.scatter.difference_stats,
                app.scatter.relief_config,
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

    let difference_is_active =
        encoding.scatter_density_mode == Some(ScatterDensityMode::FilteredDifference);
    ui.heading("Visual Encoding");
    ui.label(
        RichText::new(if difference_is_active {
            "Normalized share difference"
        } else {
            &encoding.surface_label
        })
        .strong(),
    );
    if !difference_is_active {
        ui.label(&encoding.measure_label);
    }
    ui.label(&encoding.bin_grid_label);
    if !difference_is_active {
        ui.label(&encoding.scale_label);
        ui.label(&encoding.palette_label);
        ui.label(&encoding.range_label);
    }
    let set_scatter_density_mode = encoding
        .scatter_density_mode
        .and_then(|mode| density_mode_selector(ui, mode, encoding.difference_available));
    let set_scatter_projection = encoding
        .scatter_projection
        .and_then(|projection| projection_selector(ui, projection));
    let (set_scatter_presentation, set_transform, set_point_reveal_mode, set_relief_config) =
        if difference_is_active {
            draw_difference_legend(ui, encoding.difference_stats);
            (None, None, None, None)
        } else {
            let presentation = encoding
                .scatter_presentation
                .and_then(|presentation| presentation_selector(ui, presentation));
            let transform = transform_selector(ui, encoding.encoding.transform);
            let points = encoding
                .point_reveal_mode
                .and_then(|mode| point_reveal_selector(ui, mode, encoding.point_reveal_stats));
            let relief = (encoding.scatter_presentation
                == Some(ScatterDensityPresentation::ReliefField))
            .then(|| encoding.relief_config)
            .flatten()
            .and_then(|config| show_relief_controls(ui, config));
            draw_density_legend(ui, encoding);
            (presentation, transform, points, relief)
        };

    DensityEncodingResponse {
        shown: true,
        set_transform,
        set_scatter_presentation,
        set_point_reveal_mode,
        set_scatter_projection,
        set_scatter_density_mode,
        set_relief_config,
    }
}

fn projection_selector(
    ui: &mut Ui,
    active_projection: ScatterProjection,
) -> Option<ScatterProjection> {
    let mut selected = None;
    ui.horizontal(|ui| {
        ui.label("Projection");
        for projection in [ScatterProjection::RawXY, ScatterProjection::MeanDifference] {
            if ui
                .add(segmented_button(
                    projection.label(),
                    projection == active_projection,
                ))
                .clicked()
                && projection != active_projection
            {
                selected = Some(projection);
            }
        }
    });
    selected
}

fn point_reveal_selector(
    ui: &mut Ui,
    active_mode: PointRevealMode,
    stats: Option<PointRevealStats>,
) -> Option<PointRevealMode> {
    let mut selected = None;
    ui.horizontal(|ui| {
        ui.label("Points");
        for (mode, label) in [
            (PointRevealMode::Auto, "Auto"),
            (PointRevealMode::Off, "Off"),
        ] {
            if ui
                .add(segmented_button(label, active_mode == mode))
                .clicked()
                && active_mode != mode
            {
                selected = Some(mode);
            }
        }
    });
    if let Some(stats) = stats {
        let sampled = if stats.sampled { " (sampled)" } else { "" };
        ui.label(format!(
            "Points {} of {}{}",
            stats.rendered_count, stats.eligible_count, sampled
        ));
    }
    selected
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
            ScatterDensityPresentation::ReliefField,
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
                    ScatterDensityPresentation::ReliefField => {
                        "Shade the same top-down density field with multiscale normals and bounded horizon shadows."
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
#[path = "ui_visual_encoding_tests.rs"]
mod tests;
