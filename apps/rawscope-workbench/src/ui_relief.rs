//! Bounded controls for top-down relief shading.

use egui::Ui;
use rawscope_render::{
    validate_relief_field_config, ReliefFieldConfig, MAX_RELIEF_ELEVATION_DEGREES,
    MAX_RELIEF_HEIGHT_STRENGTH, MAX_RELIEF_NORMAL_RADIUS_BINS, MIN_RELIEF_ELEVATION_DEGREES,
    MIN_RELIEF_HEIGHT_STRENGTH, MIN_RELIEF_NORMAL_RADIUS_BINS,
};

pub(crate) fn show_relief_controls(
    ui: &mut Ui,
    current: ReliefFieldConfig,
) -> Option<ReliefFieldConfig> {
    ui.label("Relief shading");
    let mut next = current;
    let mut changed = false;
    changed |= ui
        .add(
            egui::Slider::new(
                &mut next.height_strength,
                MIN_RELIEF_HEIGHT_STRENGTH..=MAX_RELIEF_HEIGHT_STRENGTH,
            )
            .text("Height"),
        )
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut next.light_azimuth_degrees, 0.0..=360.0).text("Light azimuth"))
        .changed();
    ui.collapsing("Advanced", |ui| {
        changed |= ui
            .add(
                egui::Slider::new(
                    &mut next.normal_radius_bins,
                    MIN_RELIEF_NORMAL_RADIUS_BINS..=MAX_RELIEF_NORMAL_RADIUS_BINS,
                )
                .text("Normal radius"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(
                    &mut next.light_elevation_degrees,
                    MIN_RELIEF_ELEVATION_DEGREES..=MAX_RELIEF_ELEVATION_DEGREES,
                )
                .text("Light elevation"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut next.ambient_strength, 0.0..=1.0).text("Ambient"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut next.shadow_strength, 0.0..=1.0).text("Horizon shadow"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut next.contour_strength, 0.0..=1.0).text("Contours"))
            .changed();
    });
    if ui.small_button("Reset relief").clicked() {
        return Some(ReliefFieldConfig::default());
    }
    changed
        .then(|| validate_relief_field_config(next).ok())
        .flatten()
}
