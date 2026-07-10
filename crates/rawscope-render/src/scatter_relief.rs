//! Validated top-down relief shading configuration and reference math.

use std::{error::Error, fmt};

pub const MIN_RELIEF_HEIGHT_STRENGTH: f32 = 0.25;
pub const MAX_RELIEF_HEIGHT_STRENGTH: f32 = 2.5;
pub const MIN_RELIEF_NORMAL_RADIUS_BINS: u32 = 1;
pub const MAX_RELIEF_NORMAL_RADIUS_BINS: u32 = 4;
pub const MIN_RELIEF_ELEVATION_DEGREES: f32 = 20.0;
pub const MAX_RELIEF_ELEVATION_DEGREES: f32 = 80.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReliefFieldConfig {
    pub height_strength: f32,
    pub normal_radius_bins: u32,
    pub light_azimuth_degrees: f32,
    pub light_elevation_degrees: f32,
    pub ambient_strength: f32,
    pub shadow_strength: f32,
    pub contour_strength: f32,
}

impl Default for ReliefFieldConfig {
    fn default() -> Self {
        Self {
            height_strength: 1.0,
            normal_radius_bins: 2,
            light_azimuth_degrees: 315.0,
            light_elevation_degrees: 48.0,
            ambient_strength: 0.42,
            shadow_strength: 0.28,
            contour_strength: 0.32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliefFieldConfigError {
    NonFinite,
    HeightStrengthOutOfRange,
    NormalRadiusOutOfRange,
    ElevationOutOfRange,
    UnitStrengthOutOfRange,
}

impl fmt::Display for ReliefFieldConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFinite => "relief parameters must be finite",
            Self::HeightStrengthOutOfRange => "relief height strength is outside 0.25..=2.5",
            Self::NormalRadiusOutOfRange => "relief normal radius is outside 1..=4 bins",
            Self::ElevationOutOfRange => "relief light elevation is outside 20..=80 degrees",
            Self::UnitStrengthOutOfRange => "relief ambient, shadow, and contour must be in 0..=1",
        };
        formatter.write_str(message)
    }
}

impl Error for ReliefFieldConfigError {}

pub fn validate_relief_field_config(
    mut config: ReliefFieldConfig,
) -> Result<ReliefFieldConfig, ReliefFieldConfigError> {
    let finite = [
        config.height_strength,
        config.light_azimuth_degrees,
        config.light_elevation_degrees,
        config.ambient_strength,
        config.shadow_strength,
        config.contour_strength,
    ]
    .into_iter()
    .all(f32::is_finite);
    if !finite {
        return Err(ReliefFieldConfigError::NonFinite);
    }
    if !(MIN_RELIEF_HEIGHT_STRENGTH..=MAX_RELIEF_HEIGHT_STRENGTH).contains(&config.height_strength)
    {
        return Err(ReliefFieldConfigError::HeightStrengthOutOfRange);
    }
    if !(MIN_RELIEF_NORMAL_RADIUS_BINS..=MAX_RELIEF_NORMAL_RADIUS_BINS)
        .contains(&config.normal_radius_bins)
    {
        return Err(ReliefFieldConfigError::NormalRadiusOutOfRange);
    }
    if !(MIN_RELIEF_ELEVATION_DEGREES..=MAX_RELIEF_ELEVATION_DEGREES)
        .contains(&config.light_elevation_degrees)
    {
        return Err(ReliefFieldConfigError::ElevationOutOfRange);
    }
    if ![
        config.ambient_strength,
        config.shadow_strength,
        config.contour_strength,
    ]
    .into_iter()
    .all(|value| (0.0..=1.0).contains(&value))
    {
        return Err(ReliefFieldConfigError::UnitStrengthOutOfRange);
    }
    config.light_azimuth_degrees = config.light_azimuth_degrees.rem_euclid(360.0);
    Ok(config)
}

pub fn relief_normal_from_samples(
    left: f32,
    right: f32,
    lower: f32,
    upper: f32,
    height_strength: f32,
) -> [f32; 3] {
    let x = (left - right) * height_strength;
    let y = (lower - upper) * height_strength;
    let length = (x * x + y * y + 1.0).sqrt();
    [x / length, y / length, 1.0 / length]
}

#[cfg(test)]
mod tests {
    use rawscope_core::F32Range;

    use crate::{DensityEncoding, ScatterDensityPresentation, ScatterDensityRendererConfig};

    use super::*;

    #[test]
    fn relief_defaults_are_valid_and_bounded() {
        assert_eq!(
            validate_relief_field_config(ReliefFieldConfig::default()),
            Ok(ReliefFieldConfig::default())
        );
    }

    #[test]
    fn relief_config_rejects_non_finite_values() {
        assert_eq!(
            validate_relief_field_config(ReliefFieldConfig {
                height_strength: f32::NAN,
                ..Default::default()
            }),
            Err(ReliefFieldConfigError::NonFinite)
        );
    }

    #[test]
    fn relief_config_rejects_out_of_range_values() {
        assert_eq!(
            validate_relief_field_config(ReliefFieldConfig {
                height_strength: MAX_RELIEF_HEIGHT_STRENGTH + 0.1,
                ..Default::default()
            }),
            Err(ReliefFieldConfigError::HeightStrengthOutOfRange)
        );
        assert_eq!(
            validate_relief_field_config(ReliefFieldConfig {
                normal_radius_bins: MIN_RELIEF_NORMAL_RADIUS_BINS - 1,
                ..Default::default()
            }),
            Err(ReliefFieldConfigError::NormalRadiusOutOfRange)
        );
    }

    #[test]
    fn relief_normal_is_flat_for_uniform_density() {
        assert_eq!(
            relief_normal_from_samples(0.5, 0.5, 0.5, 0.5, 2.0),
            [0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn relief_presentation_preserves_density_encoding_contract() {
        let encoding = DensityEncoding::scatter_default();
        let config = ScatterDensityRendererConfig::new(
            F32Range::new(0.0, 1.0),
            F32Range::new(0.0, 1.0),
            16,
            16,
        )
        .with_encoding(encoding)
        .with_presentation(ScatterDensityPresentation::ReliefField)
        .with_relief(ReliefFieldConfig::default());

        assert_eq!(config.encoding, encoding);
        assert_eq!(config.presentation, ScatterDensityPresentation::ReliefField);
    }
}
