//! Cohort-comparison mathematics shared by every visual-field presentation.
//!
//! The comparison contract is deliberately count based.  A renderer may choose
//! how to colour or split the resulting field, but it must not change the
//! normalized active-minus-baseline formula represented here.

use std::{error::Error, fmt};

/// Exact count context for one comparison cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferenceCellContext {
    pub baseline_count: u32,
    pub active_count: u32,
    pub baseline_share: f64,
    pub active_share: f64,
    pub signed_delta: f64,
    pub support_share: f64,
}

/// Failure while normalizing comparison cell counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonError {
    ZeroBaselineTotal,
    ZeroActiveTotal,
    NonFiniteInput,
}

impl fmt::Display for ComparisonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroBaselineTotal => "baseline cohort total must be positive",
            Self::ZeroActiveTotal => "active cohort total must be positive",
            Self::NonFiniteInput => "comparison counts and totals must be finite",
        })
    }
}

impl Error for ComparisonError {}

/// Computes normalized active-share-minus-baseline-share context for one cell.
///
/// `support_share` is an equal-weighted presentation aid.  It is intentionally
/// independent of the signed delta and must never be described as significance.
pub fn summarize_difference_cell(
    baseline_count: u32,
    baseline_total: u64,
    active_count: u32,
    active_total: u64,
) -> Result<DifferenceCellContext, ComparisonError> {
    if baseline_total == 0 {
        return Err(ComparisonError::ZeroBaselineTotal);
    }
    if active_total == 0 {
        return Err(ComparisonError::ZeroActiveTotal);
    }

    let baseline_share = f64::from(baseline_count) / baseline_total as f64;
    let active_share = f64::from(active_count) / active_total as f64;
    let signed_delta = active_share - baseline_share;
    let support_share = (baseline_share + active_share) / 2.0;
    if !baseline_share.is_finite()
        || !active_share.is_finite()
        || !signed_delta.is_finite()
        || !support_share.is_finite()
    {
        return Err(ComparisonError::NonFiniteInput);
    }

    Ok(DifferenceCellContext {
        baseline_count,
        active_count,
        baseline_share,
        active_share,
        signed_delta,
        support_share,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difference_summary_preserves_normalized_share_formula() {
        let summary = summarize_difference_cell(50, 100, 40, 50).unwrap();
        assert_eq!(summary.baseline_share, 0.5);
        assert_eq!(summary.active_share, 0.8);
        assert!((summary.signed_delta - 0.3).abs() < 1.0e-12);
        assert!((summary.support_share - 0.65).abs() < 1.0e-12);
    }

    #[test]
    fn zero_totals_make_comparison_unavailable() {
        assert_eq!(
            summarize_difference_cell(1, 0, 1, 1),
            Err(ComparisonError::ZeroBaselineTotal)
        );
        assert_eq!(
            summarize_difference_cell(1, 1, 1, 0),
            Err(ComparisonError::ZeroActiveTotal)
        );
    }

    #[test]
    fn support_visibility_does_not_change_reported_delta() {
        let sparse = summarize_difference_cell(1, 100, 2, 100).unwrap();
        let dense = summarize_difference_cell(50, 100, 51, 100).unwrap();
        assert!((sparse.signed_delta - dense.signed_delta).abs() < 1.0e-12);
        assert!(sparse.support_share < dense.support_share);
    }
}
