//! Render-side presentation state for the continuous density/point zoom.
//!
//! Selection and semantic-zoom policy live in analysis.  The renderer only
//! needs the two bounded alpha channels that a settled frame has already
//! resolved.  Keeping this adapter render-owned means a frame can be applied
//! to resident point buffers without touching row data or rebuilding a plan.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointRevealPresentationFrame {
    /// Opacity multiplier for the contextual density field.
    pub density_alpha: f32,
    /// Opacity multiplier for resident point glyphs.
    pub point_alpha: f32,
}

impl PointRevealPresentationFrame {
    /// Build a frame from semantic-zoom alphas, clamping untrusted values at
    /// the GPU boundary so NaN/Inf cannot enter a uniform buffer.
    pub fn from_alphas(density_alpha: f32, point_alpha: f32) -> Self {
        Self {
            density_alpha: bounded_alpha(density_alpha),
            point_alpha: bounded_alpha(point_alpha),
        }
    }

    /// A point-only frame preserves the legacy renderer behavior while making
    /// the point channel explicit for callers that have no density crossfade.
    pub fn point_only(point_alpha: f32) -> Self {
        Self::from_alphas(1.0, point_alpha)
    }

    /// Return the bounded density multiplier for a base renderer opacity.
    pub fn apply_density(self, base_alpha: f32) -> f32 {
        bounded_alpha(base_alpha) * self.density_alpha
    }

    /// Return the bounded glyph multiplier after the settled selection blend.
    pub fn apply_points(self, selection_blend: f32) -> f32 {
        bounded_alpha(selection_blend) * self.point_alpha
    }
}

impl From<rawscope_analysis::visual_field::SemanticZoomFrame> for PointRevealPresentationFrame {
    fn from(frame: rawscope_analysis::visual_field::SemanticZoomFrame) -> Self {
        Self::from_alphas(frame.density_alpha, frame.point_alpha)
    }
}

fn bounded_alpha(alpha: f32) -> f32 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_frame_bounds_alphas_before_gpu_upload() {
        let frame = PointRevealPresentationFrame::from_alphas(2.0, f32::NAN);

        assert_eq!(frame.density_alpha, 1.0);
        assert_eq!(frame.point_alpha, 0.0);
        assert_eq!(frame.apply_density(0.5), 0.5);
        assert_eq!(frame.apply_points(0.75), 0.0);
    }

    #[test]
    fn point_only_frame_preserves_legacy_density_and_scales_points() {
        let frame = PointRevealPresentationFrame::point_only(0.4);

        assert_eq!(frame.apply_density(0.8), 0.8);
        assert!((frame.apply_points(0.5) - 0.2).abs() < f32::EPSILON);
    }
}
