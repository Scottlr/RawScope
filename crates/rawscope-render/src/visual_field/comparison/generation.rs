//! Paired-generation identity and compatibility for cohort comparison.

use std::{error::Error, fmt, sync::Arc};

use rawscope_analysis::visual_field::{ComparisonError, DensityEncoding, DifferenceCellContext};
use rawscope_core::{F32Range, GridSize};
use rawscope_data::DatasetGeneration;

use super::super::generation::{VisualFieldGeneration, VisualFieldQuality};
use super::super::palette::ContinuousPalette;

/// Shared non-cohort semantics required by both sides of a comparison.
///
/// The cohort mask is intentionally not part of this value: baseline and
/// active fields may select different rows while retaining one row domain,
/// mapping, grid, encoding, and palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComparisonFieldSemantics {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub encoding: DensityEncoding,
    pub palette: ContinuousPalette,
}

impl ComparisonFieldSemantics {
    pub const fn new(
        x_range: F32Range,
        y_range: F32Range,
        encoding: DensityEncoding,
        palette: ContinuousPalette,
    ) -> Self {
        Self {
            x_range,
            y_range,
            encoding,
            palette,
        }
    }
}

/// Compatibility identity shared by a published baseline/active pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparisonCompatibility {
    dataset_generation: DatasetGeneration,
    device_generation: rawscope_gpu::DeviceGeneration,
    mapping: rawscope_analysis::visual_field::VisualFieldMapping,
    grid: GridSize,
    quality: VisualFieldQuality,
}

impl ComparisonCompatibility {
    pub fn new(
        baseline: &VisualFieldGeneration,
        active: &VisualFieldGeneration,
    ) -> Result<Self, ComparisonCompatibilityError> {
        Self::try_new(baseline, active)
    }

    pub fn try_new(
        baseline: &VisualFieldGeneration,
        active: &VisualFieldGeneration,
    ) -> Result<Self, ComparisonCompatibilityError> {
        let baseline_resources = baseline.resources();
        let active_resources = active.resources();
        if baseline_resources.dataset_generation() != active_resources.dataset_generation() {
            return Err(ComparisonCompatibilityError::DatasetGenerationMismatch);
        }
        if baseline_resources.device_generation() != active_resources.device_generation() {
            return Err(ComparisonCompatibilityError::DeviceGenerationMismatch);
        }
        if baseline_resources.mapping() != active_resources.mapping() {
            return Err(ComparisonCompatibilityError::MappingMismatch);
        }
        if baseline_resources.point_count() != active_resources.point_count() {
            return Err(ComparisonCompatibilityError::RowDomainMismatch);
        }
        if baseline.grid() != active.grid() {
            return Err(ComparisonCompatibilityError::GridMismatch);
        }
        if baseline.quality() != active.quality() {
            return Err(ComparisonCompatibilityError::QualityMismatch);
        }
        Ok(Self {
            dataset_generation: baseline_resources.dataset_generation(),
            device_generation: baseline_resources.device_generation(),
            mapping: baseline_resources.mapping(),
            grid: baseline.grid(),
            quality: baseline.quality(),
        })
    }

    pub fn try_new_with_semantics(
        baseline: &VisualFieldGeneration,
        active: &VisualFieldGeneration,
        baseline_semantics: ComparisonFieldSemantics,
        active_semantics: ComparisonFieldSemantics,
    ) -> Result<Self, ComparisonCompatibilityError> {
        if baseline_semantics.x_range != active_semantics.x_range {
            return Err(ComparisonCompatibilityError::XDomainMismatch);
        }
        if baseline_semantics.y_range != active_semantics.y_range {
            return Err(ComparisonCompatibilityError::YDomainMismatch);
        }
        if baseline_semantics.encoding != active_semantics.encoding {
            return Err(ComparisonCompatibilityError::EncodingMismatch);
        }
        if baseline_semantics.palette != active_semantics.palette {
            return Err(ComparisonCompatibilityError::PaletteMismatch);
        }
        Self::try_new(baseline, active)
    }

    pub const fn dataset_generation(self) -> DatasetGeneration {
        self.dataset_generation
    }

    pub const fn device_generation(self) -> rawscope_gpu::DeviceGeneration {
        self.device_generation
    }

    pub const fn mapping(self) -> rawscope_analysis::visual_field::VisualFieldMapping {
        self.mapping
    }

    pub const fn grid(self) -> GridSize {
        self.grid
    }

    pub const fn quality(self) -> VisualFieldQuality {
        self.quality
    }
}

/// Why a baseline/active pair cannot be published atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonCompatibilityError {
    DatasetGenerationMismatch,
    DeviceGenerationMismatch,
    MappingMismatch,
    RowDomainMismatch,
    GridMismatch,
    QualityMismatch,
    XDomainMismatch,
    YDomainMismatch,
    EncodingMismatch,
    PaletteMismatch,
    ZeroBaselineTotal,
    ZeroActiveTotal,
    NonFiniteMetric,
}

impl fmt::Display for ComparisonCompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DatasetGenerationMismatch => {
                "comparison fields use different dataset generations"
            }
            Self::DeviceGenerationMismatch => "comparison fields use different device generations",
            Self::MappingMismatch => "comparison fields use different axis/category mappings",
            Self::RowDomainMismatch => "comparison fields use different row domains",
            Self::GridMismatch => "comparison fields use different grid dimensions",
            Self::QualityMismatch => "comparison fields use different quality levels",
            Self::XDomainMismatch => "comparison fields use different x-axis domains",
            Self::YDomainMismatch => "comparison fields use different y-axis domains",
            Self::EncodingMismatch => {
                "comparison fields use different density transforms or normalization"
            }
            Self::PaletteMismatch => "comparison fields use different palette semantics",
            Self::ZeroBaselineTotal => "baseline cohort total must be positive",
            Self::ZeroActiveTotal => "active cohort total must be positive",
            Self::NonFiniteMetric => "comparison metrics must be finite and non-negative",
        })
    }
}

impl Error for ComparisonCompatibilityError {}

/// One immutable, paired comparison field generation.
#[derive(Debug, Clone)]
pub struct ComparisonFieldGeneration {
    pub baseline: Arc<VisualFieldGeneration>,
    pub active: Arc<VisualFieldGeneration>,
    pub baseline_total: u64,
    pub active_total: u64,
    pub shared_density_max_count: u32,
    pub max_abs_delta: f64,
    pub max_support_share: f64,
    pub compatibility: ComparisonCompatibility,
}

impl ComparisonFieldGeneration {
    pub fn new(
        baseline: Arc<VisualFieldGeneration>,
        active: Arc<VisualFieldGeneration>,
        baseline_total: u64,
        active_total: u64,
        shared_density_max_count: u32,
        max_abs_delta: f64,
        max_support_share: f64,
    ) -> Result<Self, ComparisonCompatibilityError> {
        Self::try_new(
            baseline,
            active,
            baseline_total,
            active_total,
            shared_density_max_count,
            max_abs_delta,
            max_support_share,
        )
    }

    pub fn try_new(
        baseline: Arc<VisualFieldGeneration>,
        active: Arc<VisualFieldGeneration>,
        baseline_total: u64,
        active_total: u64,
        shared_density_max_count: u32,
        max_abs_delta: f64,
        max_support_share: f64,
    ) -> Result<Self, ComparisonCompatibilityError> {
        let compatibility = ComparisonCompatibility::try_new(&baseline, &active)?;
        validate_totals_and_metrics(
            baseline_total,
            active_total,
            max_abs_delta,
            max_support_share,
        )?;
        Ok(Self {
            baseline,
            active,
            baseline_total,
            active_total,
            shared_density_max_count,
            max_abs_delta,
            max_support_share,
            compatibility,
        })
    }

    pub fn try_new_with_semantics(
        baseline: Arc<VisualFieldGeneration>,
        active: Arc<VisualFieldGeneration>,
        baseline_semantics: ComparisonFieldSemantics,
        active_semantics: ComparisonFieldSemantics,
        baseline_total: u64,
        active_total: u64,
        shared_density_max_count: u32,
        max_abs_delta: f64,
        max_support_share: f64,
    ) -> Result<Self, ComparisonCompatibilityError> {
        let compatibility = ComparisonCompatibility::try_new_with_semantics(
            &baseline,
            &active,
            baseline_semantics,
            active_semantics,
        )?;
        validate_totals_and_metrics(
            baseline_total,
            active_total,
            max_abs_delta,
            max_support_share,
        )?;
        Ok(Self {
            baseline,
            active,
            baseline_total,
            active_total,
            shared_density_max_count,
            max_abs_delta,
            max_support_share,
            compatibility,
        })
    }

    pub fn inspect_cell(
        &self,
        baseline_count: u32,
        active_count: u32,
    ) -> Result<DifferenceCellContext, ComparisonError> {
        rawscope_analysis::visual_field::summarize_difference_cell(
            baseline_count,
            self.baseline_total,
            active_count,
            self.active_total,
        )
    }
}

fn validate_totals_and_metrics(
    baseline_total: u64,
    active_total: u64,
    max_abs_delta: f64,
    max_support_share: f64,
) -> Result<(), ComparisonCompatibilityError> {
    if baseline_total == 0 {
        return Err(ComparisonCompatibilityError::ZeroBaselineTotal);
    }
    if active_total == 0 {
        return Err(ComparisonCompatibilityError::ZeroActiveTotal);
    }
    if !max_abs_delta.is_finite()
        || max_abs_delta < 0.0
        || !max_support_share.is_finite()
        || max_support_share < 0.0
    {
        return Err(ComparisonCompatibilityError::NonFiniteMetric);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visual_field::{VisualFieldDatasetGpuResources, VisualFieldViewGenerationCounter};
    use rawscope_analysis::visual_field::{VisualFieldMapping, VisualFieldProjection};
    use rawscope_core::ColumnId;
    use rawscope_data::{DatasetGenerationCounter, DatasetSchema, StoreColumnKind};
    use rawscope_gpu::DeviceGeneration;

    fn mapping() -> VisualFieldMapping {
        let schema =
            DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)])
                .unwrap();
        VisualFieldMapping::try_new(
            &schema,
            VisualFieldProjection::NumericPair {
                x: ColumnId::new(0),
                y: ColumnId::new(1),
            },
            None,
        )
        .unwrap()
    }

    fn generation(
        dataset_generation: rawscope_data::DatasetGeneration,
        device_generation: DeviceGeneration,
        grid: GridSize,
    ) -> VisualFieldGeneration {
        let resources = Arc::new(VisualFieldDatasetGpuResources::new(
            dataset_generation,
            device_generation,
            mapping(),
            100,
            400,
        ));
        let mut cohorts = rawscope_analysis::cohort::CohortGenerationCounter::default();
        let mut views = VisualFieldViewGenerationCounter::default();
        VisualFieldGeneration::new(
            resources,
            cohorts.mint(),
            views.mint(),
            grid,
            VisualFieldQuality::Exact,
        )
    }

    fn semantics(x_min: f32, x_max: f32) -> ComparisonFieldSemantics {
        ComparisonFieldSemantics::new(
            F32Range::new(x_min, x_max),
            F32Range::new(0.0, 1.0),
            DensityEncoding::scatter_default(),
            ContinuousPalette::CohortDifference,
        )
    }

    #[test]
    fn comparison_rejects_mixed_domains_and_generations() {
        let mut datasets = DatasetGenerationCounter::default();
        let dataset = datasets.mint();
        let baseline = Arc::new(generation(
            dataset,
            DeviceGeneration(1),
            GridSize::new(8, 8),
        ));
        let active = Arc::new(generation(
            dataset,
            DeviceGeneration(1),
            GridSize::new(8, 8),
        ));
        assert_eq!(
            ComparisonCompatibility::try_new_with_semantics(
                &baseline,
                &active,
                semantics(0.0, 1.0),
                semantics(0.0, 2.0),
            ),
            Err(ComparisonCompatibilityError::XDomainMismatch)
        );

        let other_dataset = datasets.mint();
        let mixed = generation(other_dataset, DeviceGeneration(1), GridSize::new(8, 8));
        assert_eq!(
            ComparisonCompatibility::try_new(&baseline, &mixed),
            Err(ComparisonCompatibilityError::DatasetGenerationMismatch)
        );
    }

    #[test]
    fn comparison_rejects_zero_cohort_totals() {
        let mut datasets = DatasetGenerationCounter::default();
        let dataset = datasets.mint();
        let baseline = Arc::new(generation(
            dataset,
            DeviceGeneration(1),
            GridSize::new(4, 4),
        ));
        let active = Arc::new(generation(
            dataset,
            DeviceGeneration(1),
            GridSize::new(4, 4),
        ));
        let error =
            ComparisonFieldGeneration::try_new(baseline, active, 0, 10, 4, 0.2, 0.3).unwrap_err();
        assert_eq!(error, ComparisonCompatibilityError::ZeroBaselineTotal);
    }
}
