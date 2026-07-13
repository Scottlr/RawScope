//! GPU-independent density transform and normalization semantics.

/// Count transform used before a visual field is normalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DensityTransform {
    Linear,
    Log1p,
}

/// Scope used to normalize transformed counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DensityNormalization {
    ViewportMax,
}

/// Analytical density semantics shared by every renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DensityEncoding {
    pub transform: DensityTransform,
    pub normalization: DensityNormalization,
}

impl DensityEncoding {
    pub const fn scatter_default() -> Self {
        Self {
            transform: DensityTransform::Log1p,
            normalization: DensityNormalization::ViewportMax,
        }
    }

    pub const fn timeline_default() -> Self {
        Self::scatter_default()
    }

    pub const fn with_transform(self, transform: DensityTransform) -> Self {
        Self { transform, ..self }
    }
}

/// Maps a density bin count to normalized color intensity.
pub fn density_intensity(count: u32, max_count: u32, transform: DensityTransform) -> f32 {
    if count == 0 || max_count == 0 {
        return 0.0;
    }

    match transform {
        DensityTransform::Linear => (count as f32 / max_count as f32).clamp(0.0, 1.0),
        DensityTransform::Log1p => {
            let count_scale = (count as f32 + 1.0).ln();
            let max_count_scale = (max_count as f32 + 1.0).ln();
            (count_scale / max_count_scale).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{density_intensity, DensityTransform};

    #[test]
    fn linear_and_log1p_match_existing_density_oracle() {
        assert_eq!(density_intensity(5, 10, DensityTransform::Linear), 0.5);
        let linear = density_intensity(1, 4, DensityTransform::Linear);
        let log_scaled = density_intensity(1, 4, DensityTransform::Log1p);
        assert!(log_scaled > linear);
        assert_eq!(density_intensity(0, 12, DensityTransform::Linear), 0.0);
        assert_eq!(density_intensity(4, 0, DensityTransform::Log1p), 0.0);
    }
}
