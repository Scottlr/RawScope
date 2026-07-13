//! Validated top-down relief shading configuration and reference math.

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

    use crate::{
        validate_relief_field_config, DensityEncoding, ReliefFieldConfig, ReliefFieldConfigError,
        ScatterDensityPresentation, ScatterDensityRendererConfig, MAX_RELIEF_HEIGHT_STRENGTH,
        MIN_RELIEF_NORMAL_RADIUS_BINS,
    };

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
