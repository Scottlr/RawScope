//! Checked inspection geometry and sparse analytical samples.

use std::{collections::BTreeMap, error::Error, fmt};

use rawscope_core::{GridSize, RowId};

use crate::density::BinIndex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F64Domain {
    min: f64,
    max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F64DomainError {
    NonFinite,
    NonIncreasing,
}

impl F64Domain {
    pub fn try_new(min: f64, max: f64) -> Result<Self, F64DomainError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(F64DomainError::NonFinite);
        }
        if max <= min {
            return Err(F64DomainError::NonIncreasing);
        }
        Ok(Self { min, max })
    }
    pub const fn min(self) -> f64 {
        self.min
    }
    pub const fn max(self) -> f64 {
        self.max
    }
    pub fn span(self) -> f64 {
        self.max - self.min
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionExtentError {
    BinOutOfBounds { x: u32, y: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectionBinExtent {
    x_bin: BinIndex,
    y_bin: BinIndex,
    x_domain: F64Domain,
    y_domain: F64Domain,
    grid: GridSize,
}

impl InspectionBinExtent {
    pub fn new(
        x_bin: BinIndex,
        y_bin: BinIndex,
        x_domain: F64Domain,
        y_domain: F64Domain,
        grid: GridSize,
    ) -> Result<Self, InspectionExtentError> {
        if x_bin.0 >= grid.width() || y_bin.0 >= grid.height() {
            return Err(InspectionExtentError::BinOutOfBounds {
                x: x_bin.0,
                y: y_bin.0,
            });
        }
        Ok(Self {
            x_bin,
            y_bin,
            x_domain,
            y_domain,
            grid,
        })
    }
    pub fn x_bin(self) -> BinIndex {
        self.x_bin
    }
    pub fn y_bin(self) -> BinIndex {
        self.y_bin
    }
    pub fn x_bounds(self) -> (f64, f64) {
        bounds(self.x_domain, self.x_bin.0, self.grid.width())
    }
    pub fn y_bounds(self) -> (f64, f64) {
        bounds(self.y_domain, self.y_bin.0, self.grid.height())
    }
    pub fn x_bounds_f32(self) -> (f32, f32) {
        let (min, max) = self.x_bounds();
        (min as f32, max as f32)
    }
    pub fn y_bounds_f32(self) -> (f32, f32) {
        let (min, max) = self.y_bounds();
        (min as f32, max as f32)
    }
}

fn bounds(domain: F64Domain, index: u32, bin_count: u32) -> (f64, f64) {
    let bin_width = domain.span() / f64::from(bin_count);
    let min = domain.min() + f64::from(index) * bin_width;
    let max = if index + 1 == bin_count {
        domain.max()
    } else {
        domain.min() + f64::from(index + 1) * bin_width
    };
    (min, max)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseInspectionSamples {
    max_per_bin: usize,
    bins: BTreeMap<u32, Vec<RowId>>,
}

impl SparseInspectionSamples {
    pub fn new(max_per_bin: usize) -> Self {
        Self {
            max_per_bin,
            bins: BTreeMap::new(),
        }
    }
    pub fn record(&mut self, bin: u32, row_id: RowId) {
        if self.max_per_bin == 0 {
            return;
        }
        let rows = self.bins.entry(bin).or_default();
        let insertion = rows.binary_search(&row_id).unwrap_or_else(|index| index);
        if insertion < rows.len() && rows[insertion] == row_id {
            return;
        }
        rows.insert(insertion, row_id);
        if rows.len() > self.max_per_bin {
            rows.pop();
        }
    }
    pub fn occupied_bin_count(&self) -> usize {
        self.bins.len()
    }
    pub fn rows_for_bin(&self, bin: u32) -> &[RowId] {
        self.bins.get(&bin).map(Vec::as_slice).unwrap_or(&[])
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DifferenceNormalizationError {
    ZeroDenominator,
    NonFinite,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferenceInspection {
    pub baseline_count: u32,
    pub active_count: u32,
    pub baseline_share: f64,
    pub active_share: f64,
    pub delta: f64,
}

pub fn inspect_difference(
    baseline_count: u32,
    active_count: u32,
    baseline_total: u64,
    active_total: u64,
) -> Result<DifferenceInspection, DifferenceNormalizationError> {
    let summary = crate::visual_field::summarize_difference_cell(
        baseline_count,
        baseline_total,
        active_count,
        active_total,
    )
    .map_err(|error| match error {
        crate::visual_field::ComparisonError::ZeroBaselineTotal
        | crate::visual_field::ComparisonError::ZeroActiveTotal => {
            DifferenceNormalizationError::ZeroDenominator
        }
        crate::visual_field::ComparisonError::NonFiniteInput => {
            DifferenceNormalizationError::NonFinite
        }
    })?;
    Ok(DifferenceInspection {
        baseline_count: summary.baseline_count,
        active_count: summary.active_count,
        baseline_share: summary.baseline_share,
        active_share: summary.active_share,
        delta: summary.signed_delta,
    })
}

pub fn normalize_difference(
    numerator: f64,
    denominator: f64,
) -> Result<f64, DifferenceNormalizationError> {
    if !numerator.is_finite() || !denominator.is_finite() {
        return Err(DifferenceNormalizationError::NonFinite);
    }
    if denominator == 0.0 {
        return Err(DifferenceNormalizationError::ZeroDenominator);
    }
    Ok(numerator / denominator)
}

impl fmt::Display for F64DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFinite => "inspection domain values must be finite",
            Self::NonIncreasing => "inspection domain maximum must exceed minimum",
        })
    }
}
impl Error for F64DomainError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extent_keeps_precise_large_domain() {
        let extent = InspectionBinExtent::new(
            BinIndex(1),
            BinIndex(0),
            F64Domain::try_new(9_007_199_254_740_992.0, 9_007_199_254_740_994.0).unwrap(),
            F64Domain::try_new(0.0, 2.0).unwrap(),
            GridSize::try_new(2, 2).unwrap(),
        )
        .unwrap();
        assert_eq!(extent.x_bounds().0, 9_007_199_254_740_993.0);
        assert!(extent.x_bounds_f32().0.is_finite());
    }
    #[test]
    fn sparse_samples_are_lazy_and_deterministic() {
        let mut samples = SparseInspectionSamples::new(2);
        samples.record(7, RowId(9));
        samples.record(7, RowId(2));
        samples.record(7, RowId(5));
        samples.record(11, RowId(3));
        assert_eq!(samples.occupied_bin_count(), 2);
        assert_eq!(samples.rows_for_bin(7), &[RowId(2), RowId(5)]);
    }
    #[test]
    fn difference_normalization_rejects_invalid_denominators() {
        assert_eq!(
            normalize_difference(1.0, 0.0),
            Err(DifferenceNormalizationError::ZeroDenominator)
        );
        assert_eq!(
            normalize_difference(f64::NAN, 1.0),
            Err(DifferenceNormalizationError::NonFinite)
        );
        assert_eq!(
            inspect_difference(1, 1, 0, 1),
            Err(DifferenceNormalizationError::ZeroDenominator)
        );
    }
}
