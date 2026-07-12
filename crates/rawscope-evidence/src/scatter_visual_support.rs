//! Semantic visual-query support types shared by scatter evidence contracts.

use std::{error::Error, fmt};

pub const INSPECTION_NEIGHBORHOOD_RADIUS_BINS: u32 = 1;
pub const MIN_RELIEF_HEIGHT_STRENGTH: f32 = 0.25;
pub const MAX_RELIEF_HEIGHT_STRENGTH: f32 = 2.5;
pub const MIN_RELIEF_NORMAL_RADIUS_BINS: u32 = 1;
pub const MAX_RELIEF_NORMAL_RADIUS_BINS: u32 = 4;
pub const MIN_RELIEF_ELEVATION_DEGREES: f32 = 20.0;
pub const MAX_RELIEF_ELEVATION_DEGREES: f32 = 80.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointRevealMode {
    Off,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScatterDensityMode {
    #[default]
    AbsoluteDensity,
    FilteredDifference,
}

impl ScatterDensityMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AbsoluteDensity => "Absolute",
            Self::FilteredDifference => "Filtered difference",
        }
    }
}

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFinite => "relief parameters must be finite",
            Self::HeightStrengthOutOfRange => "relief height strength is outside 0.25..=2.5",
            Self::NormalRadiusOutOfRange => "relief normal radius is outside 1..=4 bins",
            Self::ElevationOutOfRange => "relief light elevation is outside 20..=80 degrees",
            Self::UnitStrengthOutOfRange => "relief ambient, shadow, and contour must be in 0..=1",
        };
        f.write_str(message)
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
    if [
        config.ambient_strength,
        config.shadow_strength,
        config.contour_strength,
    ]
    .into_iter()
    .any(|value| !(0.0..=1.0).contains(&value))
    {
        return Err(ReliefFieldConfigError::UnitStrengthOutOfRange);
    }
    config.light_azimuth_degrees = config.light_azimuth_degrees.rem_euclid(360.0);
    Ok(config)
}
