//! Normalized active-versus-baseline density differences.

use std::{error::Error, fmt};

pub const DIFFERENCE_FIXED_POINT_SCALE: u32 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferencePalette {
    TealNeutralCoral,
}

impl DifferencePalette {
    pub const fn legend_rgb(self) -> [[u8; 3]; 3] {
        match self {
            Self::TealNeutralCoral => [[24, 144, 153], [36, 42, 48], [232, 112, 96]],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferenceDensityConfig {
    pub palette: DifferencePalette,
}

impl Default for DifferenceDensityConfig {
    fn default() -> Self {
        Self {
            palette: DifferencePalette::TealNeutralCoral,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferenceInspection {
    pub baseline_count: u32,
    pub active_count: u32,
    pub baseline_share: f64,
    pub active_share: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferenceDensityStats {
    pub baseline_total: u64,
    pub active_total: u64,
    pub max_abs_delta: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DifferenceDensityGrid {
    pub deltas: Vec<f64>,
    pub stats: DifferenceDensityStats,
}

impl DifferenceDensityGrid {
    pub fn inspection(
        &self,
        index: usize,
        baseline_counts: &[u32],
        active_counts: &[u32],
    ) -> Option<DifferenceInspection> {
        let baseline_count = *baseline_counts.get(index)?;
        let active_count = *active_counts.get(index)?;
        Some(difference_inspection(
            baseline_count,
            active_count,
            self.stats.baseline_total,
            self.stats.active_total,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceDensityError {
    GridLengthMismatch { baseline: usize, active: usize },
    ZeroBaselineTotal,
    ZeroActiveTotal,
}

impl fmt::Display for DifferenceDensityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridLengthMismatch { baseline, active } => write!(
                formatter,
                "difference density grid length mismatch: baseline {baseline}, active {active}"
            ),
            Self::ZeroBaselineTotal => write!(formatter, "baseline cohort total must be positive"),
            Self::ZeroActiveTotal => write!(formatter, "active cohort total must be positive"),
        }
    }
}

impl Error for DifferenceDensityError {}

pub fn normalized_difference_density(
    baseline_counts: &[u32],
    active_counts: &[u32],
    baseline_total: u64,
    active_total: u64,
) -> Result<DifferenceDensityGrid, DifferenceDensityError> {
    validate_inputs(
        baseline_counts.len(),
        active_counts.len(),
        baseline_total,
        active_total,
    )?;
    let deltas = baseline_counts
        .iter()
        .zip(active_counts)
        .map(|(&baseline, &active)| {
            difference_inspection(baseline, active, baseline_total, active_total).delta
        })
        .collect::<Vec<_>>();
    let max_abs_delta = deltas
        .iter()
        .fold(0.0_f64, |max, delta| max.max(delta.abs()));
    Ok(DifferenceDensityGrid {
        deltas,
        stats: DifferenceDensityStats {
            baseline_total,
            active_total,
            max_abs_delta,
        },
    })
}

pub fn difference_inspection(
    baseline_count: u32,
    active_count: u32,
    baseline_total: u64,
    active_total: u64,
) -> DifferenceInspection {
    let baseline_share = f64::from(baseline_count) / baseline_total as f64;
    let active_share = f64::from(active_count) / active_total as f64;
    DifferenceInspection {
        baseline_count,
        active_count,
        baseline_share,
        active_share,
        delta: active_share - baseline_share,
    }
}

pub fn fixed_point_max_abs_delta(delta: f64) -> u32 {
    (delta.abs() * f64::from(DIFFERENCE_FIXED_POINT_SCALE))
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

fn validate_inputs(
    baseline_len: usize,
    active_len: usize,
    baseline_total: u64,
    active_total: u64,
) -> Result<(), DifferenceDensityError> {
    if baseline_len != active_len {
        return Err(DifferenceDensityError::GridLengthMismatch {
            baseline: baseline_len,
            active: active_len,
        });
    }
    if baseline_total == 0 {
        return Err(DifferenceDensityError::ZeroBaselineTotal);
    }
    if active_total == 0 {
        return Err(DifferenceDensityError::ZeroActiveTotal);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_distributions_have_zero_difference() {
        let grid = normalized_difference_density(&[1, 3], &[2, 6], 4, 8).unwrap();
        assert_eq!(grid.deltas, vec![0.0, 0.0]);
        assert_eq!(grid.stats.max_abs_delta, 0.0);
    }

    #[test]
    fn share_delta_handles_unequal_cohort_sizes() {
        let grid = normalized_difference_density(&[50, 50], &[40, 10], 100, 50).unwrap();
        assert!((grid.deltas[0] - 0.3).abs() < f64::EPSILON * 2.0);
        assert!((grid.deltas[1] + 0.3).abs() < f64::EPSILON * 2.0);
    }

    #[test]
    fn difference_domain_is_symmetric_around_zero() {
        let grid = normalized_difference_density(&[90, 10], &[20, 30], 100, 50).unwrap();
        assert_eq!(grid.stats.max_abs_delta, 0.5);
        assert!(grid.deltas.iter().all(|delta| delta.abs() <= 0.5));
    }

    #[test]
    fn zero_total_rejects_difference_mode() {
        assert_eq!(
            normalized_difference_density(&[1], &[1], 1, 0),
            Err(DifferenceDensityError::ZeroActiveTotal)
        );
    }

    #[test]
    fn difference_inspection_matches_grid_formula() {
        let grid = normalized_difference_density(&[25], &[20], 100, 40).unwrap();
        let inspection = grid.inspection(0, &[25], &[20]).unwrap();
        assert_eq!(inspection.baseline_share, 0.25);
        assert_eq!(inspection.active_share, 0.5);
        assert_eq!(inspection.delta, grid.deltas[0]);
    }

    #[test]
    fn fixed_point_error_is_bounded_by_one_unit() {
        let delta = 0.123_456_789_4;
        let encoded = fixed_point_max_abs_delta(delta);
        let decoded = f64::from(encoded) / f64::from(DIFFERENCE_FIXED_POINT_SCALE);
        assert!((decoded - delta).abs() <= 1.0 / f64::from(DIFFERENCE_FIXED_POINT_SCALE));
    }
}
