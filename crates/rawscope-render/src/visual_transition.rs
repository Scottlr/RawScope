//! Bounded timing and color-composition contracts for visual transitions.

pub const MAX_VISUAL_TRANSITION_DURATION_MS: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualTransitionConfig {
    pub duration_ms: u32,
    pub reduced_motion: bool,
}

impl Default for VisualTransitionConfig {
    fn default() -> Self {
        Self {
            duration_ms: 180,
            reduced_motion: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualTransitionConfigError {
    DurationOutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    DensityRefresh,
    ProjectionChange,
    PresentationChange,
    DifferenceModeChange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionProgress(pub f32);

pub fn validate_transition_config(
    config: VisualTransitionConfig,
) -> Result<VisualTransitionConfig, VisualTransitionConfigError> {
    if config.duration_ms > MAX_VISUAL_TRANSITION_DURATION_MS {
        return Err(VisualTransitionConfigError::DurationOutOfRange);
    }
    Ok(config)
}

pub fn transition_progress(elapsed_ms: u64, config: VisualTransitionConfig) -> TransitionProgress {
    if config.reduced_motion || config.duration_ms == 0 {
        return TransitionProgress(1.0);
    }
    TransitionProgress((elapsed_ms as f64 / f64::from(config.duration_ms)).clamp(0.0, 1.0) as f32)
}

pub fn ease_out_cubic(progress: TransitionProgress) -> f32 {
    let progress = progress.0.clamp(0.0, 1.0);
    1.0 - (1.0 - progress).powi(3)
}

pub fn semantic_color_crossfade(from: [f32; 3], to: [f32; 3], alpha: f32) -> [f32; 3] {
    let alpha = alpha.clamp(0.0, 1.0);
    std::array::from_fn(|index| from[index] + (to[index] - from[index]) * alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_progress_is_monotonic_and_bounded() {
        let config = VisualTransitionConfig::default();
        let samples = [0, 45, 90, 180, 360].map(|elapsed| transition_progress(elapsed, config).0);
        assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[3], 1.0);
        assert_eq!(samples[4], 1.0);
    }

    #[test]
    fn reduced_motion_completes_immediately() {
        assert_eq!(
            transition_progress(
                0,
                VisualTransitionConfig {
                    reduced_motion: true,
                    ..Default::default()
                }
            ),
            TransitionProgress(1.0)
        );
    }

    #[test]
    fn transition_duration_rejects_values_above_bound() {
        assert_eq!(
            validate_transition_config(VisualTransitionConfig {
                duration_ms: MAX_VISUAL_TRANSITION_DURATION_MS + 1,
                reduced_motion: false,
            }),
            Err(VisualTransitionConfigError::DurationOutOfRange)
        );
    }

    #[test]
    fn semantic_mode_change_uses_color_crossfade() {
        let colour = semantic_color_crossfade([0.0, 0.2, 0.4], [1.0, 0.6, 0.2], 0.5);
        let expected = [0.5, 0.4, 0.3];
        assert!(colour
            .into_iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON * 2.0));
    }
}
