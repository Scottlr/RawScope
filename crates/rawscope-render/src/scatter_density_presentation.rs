//! Scatter-specific presentation modes layered over exact GPU density bins.

use serde::{Deserialize, Serialize};

/// Fragment-stage presentation applied to the exact scatter-density count grid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScatterDensityPresentation {
    /// One constant colour per exact density bin.
    #[default]
    ExactCells,
    /// Bilinear field reconstruction with density contours and gradient relief.
    TopographicField,
}

impl ScatterDensityPresentation {
    /// Shader identifier sent in scatter render uniforms.
    pub fn shader_id(self) -> u32 {
        match self {
            Self::ExactCells => 0,
            Self::TopographicField => 1,
        }
    }

    /// Compact control label for the workbench.
    pub fn label(self) -> &'static str {
        match self {
            Self::ExactCells => "Cells",
            Self::TopographicField => "Topographic",
        }
    }

    /// Stable analyst-facing label for evidence and visual context.
    pub fn evidence_label(self) -> &'static str {
        match self {
            Self::ExactCells => "exact cells",
            Self::TopographicField => "topographic field",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScatterDensityPresentation;

    #[test]
    fn presentation_shader_ids_are_stable() {
        assert_eq!(ScatterDensityPresentation::ExactCells.shader_id(), 0);
        assert_eq!(ScatterDensityPresentation::TopographicField.shader_id(), 1);
    }
}
