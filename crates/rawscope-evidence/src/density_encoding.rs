//! Density color encoding contracts shared by renderers, UI, and evidence.

use serde::{Deserialize, Serialize};

/// Supported transforms for mapping bin counts into color intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DensityTransform {
    Linear,
    Log1p,
}

impl DensityTransform {
    /// Shader identifier used by density render uniforms.
    pub fn shader_id(self) -> u32 {
        match self {
            Self::Linear => 0,
            Self::Log1p => 1,
        }
    }

    /// Analyst-facing label for the transform.
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "linear(count)",
            Self::Log1p => "log1p(count)",
        }
    }
}

/// Density palettes currently owned by RawScope renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DensityPalette {
    ScatterSequential,
    TimelineSequential,
}

impl DensityPalette {
    /// Shader identifier used by density render uniforms.
    pub fn shader_id(self) -> u32 {
        match self {
            Self::ScatterSequential => 0,
            Self::TimelineSequential => 1,
        }
    }

    /// Compact label for evidence and UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::ScatterSequential => "scatter sequential",
            Self::TimelineSequential => "timeline sequential",
        }
    }
}

/// Scope used to normalize transformed bin counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DensityNormalization {
    ViewportMax,
}

impl DensityNormalization {
    /// Analyst-facing label for the normalization scope.
    pub fn label(self) -> &'static str {
        match self {
            Self::ViewportMax => "viewport max",
        }
    }
}

/// Full density color encoding sent to renderers and projected into UI/evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DensityEncoding {
    pub transform: DensityTransform,
    pub palette: DensityPalette,
    pub normalization: DensityNormalization,
}

impl DensityEncoding {
    /// Default encoding for scatter density surfaces.
    pub fn scatter_default() -> Self {
        Self {
            transform: DensityTransform::Log1p,
            palette: DensityPalette::ScatterSequential,
            normalization: DensityNormalization::ViewportMax,
        }
    }

    /// Default encoding for timeline density surfaces.
    pub fn timeline_default() -> Self {
        Self {
            transform: DensityTransform::Log1p,
            palette: DensityPalette::TimelineSequential,
            normalization: DensityNormalization::ViewportMax,
        }
    }

    /// Returns this encoding with a new transform while preserving palette and normalization.
    pub fn with_transform(self, transform: DensityTransform) -> Self {
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
    fn linear_density_intensity_maps_half_count_to_half() {
        assert_eq!(density_intensity(5, 10, DensityTransform::Linear), 0.5);
    }

    #[test]
    fn log1p_density_intensity_keeps_low_counts_visible() {
        let linear_midpoint = density_intensity(1, 4, DensityTransform::Linear);
        let log_scaled = density_intensity(1, 4, DensityTransform::Log1p);

        assert!(log_scaled > linear_midpoint);
        assert!(log_scaled < 1.0);
    }

    #[test]
    fn density_intensity_maps_empty_bins_to_zero() {
        assert_eq!(density_intensity(0, 12, DensityTransform::Linear), 0.0);
        assert_eq!(density_intensity(4, 0, DensityTransform::Log1p), 0.0);
    }
}
