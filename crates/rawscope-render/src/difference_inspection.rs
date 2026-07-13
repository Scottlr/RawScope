//! Settled active-versus-baseline inspection semantics for difference density.

use std::collections::BTreeMap;

use crate::difference_density::difference_inspection;
use crate::{fixed_point_max_abs_delta, normalized_difference_density, DifferenceDensityError};
use rawscope_analysis::inspection::DifferenceInspection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceDirection {
    MoreCommonInActive,
    LessCommonInActive,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferenceInspectionSummary {
    pub baseline_count: u32,
    pub active_count: u32,
    pub baseline_total: u64,
    pub active_total: u64,
    pub baseline_share: f64,
    pub active_share: f64,
    pub share_delta: f64,
    pub support_share: f64,
    pub direction: DifferenceDirection,
    pub absolute_delta_percentile: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferenceInspectionDistribution {
    pub baseline_total: u64,
    pub active_total: u64,
    pub meaningful_bin_count: u32,
    absolute_delta_histogram: Vec<(u32, u32)>,
}

impl DifferenceInspectionDistribution {
    pub fn summarize_counts(
        &self,
        baseline_count: u32,
        active_count: u32,
    ) -> Option<DifferenceInspectionSummary> {
        let inspection = difference_inspection(
            baseline_count,
            active_count,
            self.baseline_total,
            self.active_total,
        )
        .ok()?;
        let direction = if inspection.delta > 0.0 {
            DifferenceDirection::MoreCommonInActive
        } else if inspection.delta < 0.0 {
            DifferenceDirection::LessCommonInActive
        } else {
            DifferenceDirection::Unchanged
        };
        Some(DifferenceInspectionSummary {
            baseline_count,
            active_count,
            baseline_total: self.baseline_total,
            active_total: self.active_total,
            baseline_share: inspection.baseline_share,
            active_share: inspection.active_share,
            share_delta: inspection.delta,
            support_share: (inspection.baseline_share + inspection.active_share) * 0.5,
            direction,
            absolute_delta_percentile: self.absolute_delta_percentile(inspection),
        })
    }

    fn absolute_delta_percentile(&self, inspection: DifferenceInspection) -> Option<f64> {
        if inspection.delta == 0.0 || self.meaningful_bin_count == 0 {
            return None;
        }
        let fixed_point_delta = fixed_point_max_abs_delta(inspection.delta);
        let at_or_below_count = self
            .absolute_delta_histogram
            .iter()
            .take_while(|(delta, _)| *delta <= fixed_point_delta)
            .map(|(_, frequency)| u64::from(*frequency))
            .sum::<u64>();
        Some(at_or_below_count as f64 / self.meaningful_bin_count as f64)
    }
}

pub fn build_difference_inspection_distribution(
    baseline_counts: &[u32],
    active_counts: &[u32],
    baseline_total: u64,
    active_total: u64,
) -> Result<DifferenceInspectionDistribution, DifferenceDensityError> {
    let difference_grid = normalized_difference_density(
        baseline_counts,
        active_counts,
        baseline_total,
        active_total,
    )?;
    let mut histogram = BTreeMap::<u32, u32>::new();
    let mut meaningful_bin_count = 0_u32;
    for ((&baseline_count, &active_count), &delta) in baseline_counts
        .iter()
        .zip(active_counts)
        .zip(&difference_grid.deltas)
    {
        if baseline_count == 0 && active_count == 0 {
            continue;
        }
        meaningful_bin_count = meaningful_bin_count.saturating_add(1);
        *histogram
            .entry(fixed_point_max_abs_delta(delta))
            .or_default() += 1;
    }
    Ok(DifferenceInspectionDistribution {
        baseline_total,
        active_total,
        meaningful_bin_count,
        absolute_delta_histogram: histogram.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difference_summary_uses_active_minus_full_baseline_share() {
        let distribution = build(&[50, 50], &[40, 10], 100, 50);

        let summary = distribution.summarize_counts(40, 40).unwrap();

        assert_eq!(summary.baseline_share, 0.4);
        assert_eq!(summary.active_share, 0.8);
        assert_eq!(summary.share_delta, 0.4);
        assert_eq!(summary.baseline_total, 100);
        assert_eq!(summary.active_total, 50);
        assert!((summary.support_share - 0.6).abs() < 1.0e-12);
        assert_eq!(summary.direction, DifferenceDirection::MoreCommonInActive);
    }

    #[test]
    fn difference_direction_matches_positive_negative_and_zero_delta() {
        let distribution = build(&[1, 1, 1], &[2, 0, 1], 3, 3);

        assert_eq!(
            distribution.summarize_counts(1, 2).unwrap().direction,
            DifferenceDirection::MoreCommonInActive
        );
        assert_eq!(
            distribution.summarize_counts(1, 0).unwrap().direction,
            DifferenceDirection::LessCommonInActive
        );
        assert_eq!(
            distribution.summarize_counts(1, 1).unwrap().direction,
            DifferenceDirection::Unchanged
        );
    }

    #[test]
    fn difference_strength_percentile_ranks_absolute_delta() {
        let distribution = build(&[2, 1, 0], &[1, 1, 1], 4, 2);

        assert_eq!(
            distribution
                .summarize_counts(2, 1)
                .unwrap()
                .absolute_delta_percentile,
            None
        );
        assert_eq!(
            distribution
                .summarize_counts(1, 1)
                .unwrap()
                .absolute_delta_percentile,
            Some(2.0 / 3.0)
        );
        assert_eq!(
            distribution
                .summarize_counts(0, 1)
                .unwrap()
                .absolute_delta_percentile,
            Some(1.0)
        );
    }

    #[test]
    fn baseline_only_cell_remains_inspectable() {
        let distribution = build(&[5, 0], &[0, 5], 10, 10);

        let summary = distribution.summarize_counts(5, 0).unwrap();

        assert_eq!(summary.active_count, 0);
        assert_eq!(summary.baseline_count, 5);
        assert_eq!(summary.direction, DifferenceDirection::LessCommonInActive);
        assert!(summary.share_delta < 0.0);
    }

    #[test]
    fn unequal_cohort_sizes_do_not_compare_raw_counts() {
        let distribution = build(&[50, 50], &[40, 10], 100, 50);

        let summary = distribution.summarize_counts(50, 40).unwrap();

        assert_eq!(summary.baseline_count, 50);
        assert_eq!(summary.active_count, 40);
        assert!((summary.share_delta - 0.3).abs() < f64::EPSILON * 4.0);
    }

    #[test]
    fn zero_cohort_totals_are_rejected_explicitly() {
        assert_eq!(
            build_difference_inspection_distribution(&[1], &[1], 0, 1),
            Err(DifferenceDensityError::ZeroBaselineTotal)
        );
        assert_eq!(
            build_difference_inspection_distribution(&[1], &[1], 1, 0),
            Err(DifferenceDensityError::ZeroActiveTotal)
        );
    }

    fn build(
        baseline_counts: &[u32],
        active_counts: &[u32],
        baseline_total: u64,
        active_total: u64,
    ) -> DifferenceInspectionDistribution {
        build_difference_inspection_distribution(
            baseline_counts,
            active_counts,
            baseline_total,
            active_total,
        )
        .unwrap()
    }
}
