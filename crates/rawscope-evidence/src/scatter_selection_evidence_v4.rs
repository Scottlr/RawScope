//! Additive scatter evidence v4 with complete semantic visual-query context.

use std::{error::Error, fmt};

use rawscope_core::{F32Range, RowId};
use rawscope_data::{DatasetFilter, DatasetIdentity, DatasetProfileId, ScatterProjection};

use crate::{
    DensityEncoding, PointRevealMode, ReliefFieldConfig, ScatterAggregateEvidenceContext,
    ScatterDensityMode, ScatterDensityPresentation, ScatterSelectionComparison,
    ScatterSelectionEvidenceV3, SelectedPointSampleV2, SelectedSourceRowSample,
};

pub const SCATTER_SELECTION_EVIDENCE_V4_SCHEMA_VERSION: u32 = 4;
pub const DIFFERENCE_FORMULA_ID: &str = "active_share_minus_full_baseline_share";
pub const DIFFERENCE_BASELINE_ID: &str = "full_dataset";

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionEvidenceV4 {
    pub schema_version: u32,
    pub dataset_identity: DatasetIdentity,
    pub active_dataset_profile: Option<DatasetProfileId>,
    pub visual_query: ScatterVisualQueryV4,
    pub cohort: ScatterCohortEvidence,
    pub selected_row_count: usize,
    pub selected_percentage: f32,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_record_sample: Vec<SelectedPointSampleV2>,
    pub selected_source_column_names: Vec<String>,
    pub selected_source_row_sample: Vec<SelectedSourceRowSample>,
    pub comparison: ScatterSelectionComparison,
    pub aggregate_context: ScatterAggregateEvidenceContext,
    pub pinned_inspection: Option<PinnedScatterInspectionEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterVisualQueryV4 {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub grid_width: u32,
    pub grid_height: u32,
    pub projection: ScatterProjection,
    pub filters: Vec<DatasetFilter>,
    pub density_mode: ScatterDensityMode,
    pub density_encoding: DensityEncoding,
    pub density_presentation: ScatterDensityPresentation,
    pub difference: Option<DifferenceDensityEvidenceConfig>,
    pub point_reveal: PointRevealEvidence,
    pub relief: Option<ReliefFieldConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterCohortEvidence {
    pub full_row_count: usize,
    pub included_row_count: usize,
    pub excluded_row_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointRevealEvidence {
    pub mode: PointRevealMode,
    pub eligible_count: usize,
    pub rendered_count: usize,
    pub sampled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DifferenceDensityEvidenceConfig {
    pub formula: &'static str,
    pub baseline: &'static str,
    pub baseline_total: u64,
    pub active_total: u64,
    pub max_abs_delta: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PinnedScatterInspectionEvidence {
    pub bin_x: u32,
    pub bin_y: u32,
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub row_count: u32,
    pub row_id_sample: Vec<RowId>,
    pub sample_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterSelectionEvidenceV4Error {
    InvalidSchemaVersion,
    IncoherentCohort,
    DifferenceRequiresFilter,
    DifferenceConfigMismatch,
    DifferenceCannotUseRelief,
    DifferenceCannotRevealPoints,
    ReliefConfigMismatch,
    PointRevealOffHasRenderedPoints,
    DifferenceContractMismatch,
    InvalidReliefConfig,
    InvalidPointRevealCounts,
    InvalidPinnedInspection,
    InvalidSelectedPercentage,
    InvalidSelectedSample,
}

impl fmt::Display for ScatterSelectionEvidenceV4Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidSchemaVersion => "scatter evidence v4 schema version must be exactly 4",
            Self::IncoherentCohort => "scatter evidence cohort counts do not add up",
            Self::DifferenceRequiresFilter => "difference evidence requires an active filter",
            Self::DifferenceConfigMismatch => "difference mode and config must be present together",
            Self::DifferenceCannotUseRelief => {
                "difference evidence cannot claim relief presentation"
            }
            Self::DifferenceCannotRevealPoints => {
                "difference evidence cannot claim rendered points"
            }
            Self::ReliefConfigMismatch => {
                "relief config must be present only for relief presentation"
            }
            Self::PointRevealOffHasRenderedPoints => {
                "point reveal off cannot claim rendered points"
            }
            Self::DifferenceContractMismatch => {
                "difference evidence formula, totals, or domain are incoherent"
            }
            Self::InvalidReliefConfig => "relief evidence config is invalid",
            Self::InvalidPointRevealCounts => "point reveal rendered count exceeds eligible count",
            Self::InvalidPinnedInspection => "pinned inspection sample disclosure is incoherent",
            Self::InvalidSelectedPercentage => "scatter evidence selected percentage is incoherent",
            Self::InvalidSelectedSample => "scatter evidence selected sample is incoherent",
        };
        formatter.write_str(message)
    }
}

impl Error for ScatterSelectionEvidenceV4Error {}

impl ScatterSelectionEvidenceV4 {
    pub fn from_v3(
        evidence: &ScatterSelectionEvidenceV3,
        visual_query: ScatterVisualQueryV4,
        cohort: ScatterCohortEvidence,
        pinned_inspection: Option<PinnedScatterInspectionEvidence>,
    ) -> Result<Self, ScatterSelectionEvidenceV4Error> {
        validate_visual_query(
            &visual_query,
            &cohort,
            evidence.dataset_identity.row_count,
            evidence.selected_row_count,
        )?;
        if pinned_inspection.as_ref().is_some_and(|pin| {
            pin.row_id_sample.len() > pin.sample_limit
                || pin.row_id_sample.len() > pin.row_count as usize
        }) {
            return Err(ScatterSelectionEvidenceV4Error::InvalidPinnedInspection);
        }
        let result = Self {
            schema_version: SCATTER_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
            dataset_identity: evidence.dataset_identity.clone(),
            active_dataset_profile: evidence.active_dataset_profile,
            visual_query,
            cohort,
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            selected_row_id_sample: evidence.selected_row_id_sample.clone(),
            selected_record_sample: evidence.selected_record_sample.clone(),
            selected_source_column_names: evidence.selected_source_column_names.clone(),
            selected_source_row_sample: evidence.selected_source_row_sample.clone(),
            comparison: evidence.comparison.clone(),
            aggregate_context: evidence.aggregate_context.clone(),
            pinned_inspection,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), ScatterSelectionEvidenceV4Error> {
        if self.schema_version != SCATTER_SELECTION_EVIDENCE_V4_SCHEMA_VERSION {
            return Err(ScatterSelectionEvidenceV4Error::InvalidSchemaVersion);
        }
        validate_visual_query(
            &self.visual_query,
            &self.cohort,
            self.dataset_identity.row_count,
            self.selected_row_count,
        )?;
        if !self.selected_percentage.is_finite()
            || !(0.0..=100.0).contains(&self.selected_percentage)
        {
            return Err(ScatterSelectionEvidenceV4Error::InvalidSelectedPercentage);
        }
        if self.selected_row_id_sample.len() > self.selected_row_count
            || self.selected_record_sample.len() > self.selected_row_count
            || self.selected_source_row_sample.len() > self.selected_row_count
        {
            return Err(ScatterSelectionEvidenceV4Error::InvalidSelectedSample);
        }
        if self.pinned_inspection.as_ref().is_some_and(|pin| {
            pin.row_id_sample.len() > pin.sample_limit
                || pin.row_id_sample.len() > pin.row_count as usize
        }) {
            return Err(ScatterSelectionEvidenceV4Error::InvalidPinnedInspection);
        }
        Ok(())
    }
}

fn validate_visual_query(
    query: &ScatterVisualQueryV4,
    cohort: &ScatterCohortEvidence,
    dataset_row_count: usize,
    selected_row_count: usize,
) -> Result<(), ScatterSelectionEvidenceV4Error> {
    if cohort.included_row_count + cohort.excluded_row_count != cohort.full_row_count
        || cohort.full_row_count != dataset_row_count
        || selected_row_count > cohort.included_row_count
    {
        return Err(ScatterSelectionEvidenceV4Error::IncoherentCohort);
    }
    let difference_active = query.density_mode == ScatterDensityMode::FilteredDifference;
    if difference_active && query.filters.is_empty() {
        return Err(ScatterSelectionEvidenceV4Error::DifferenceRequiresFilter);
    }
    if difference_active != query.difference.is_some() {
        return Err(ScatterSelectionEvidenceV4Error::DifferenceConfigMismatch);
    }
    if query.difference.as_ref().is_some_and(|difference| {
        difference.formula != DIFFERENCE_FORMULA_ID
            || difference.baseline != DIFFERENCE_BASELINE_ID
            || difference.baseline_total != cohort.full_row_count as u64
            || difference.active_total != cohort.included_row_count as u64
            || !difference.max_abs_delta.is_finite()
            || difference.max_abs_delta < 0.0
    }) {
        return Err(ScatterSelectionEvidenceV4Error::DifferenceContractMismatch);
    }
    if difference_active && query.density_presentation == ScatterDensityPresentation::ReliefField {
        return Err(ScatterSelectionEvidenceV4Error::DifferenceCannotUseRelief);
    }
    if difference_active && query.point_reveal.rendered_count > 0 {
        return Err(ScatterSelectionEvidenceV4Error::DifferenceCannotRevealPoints);
    }
    let relief_active = query.density_presentation == ScatterDensityPresentation::ReliefField;
    if relief_active != query.relief.is_some() {
        return Err(ScatterSelectionEvidenceV4Error::ReliefConfigMismatch);
    }
    if query
        .relief
        .is_some_and(|relief| crate::validate_relief_field_config(relief).is_err())
    {
        return Err(ScatterSelectionEvidenceV4Error::InvalidReliefConfig);
    }
    if query.point_reveal.rendered_count > query.point_reveal.eligible_count {
        return Err(ScatterSelectionEvidenceV4Error::InvalidPointRevealCounts);
    }
    if query.point_reveal.mode == PointRevealMode::Off && query.point_reveal.rendered_count > 0 {
        return Err(ScatterSelectionEvidenceV4Error::PointRevealOffHasRenderedPoints);
    }
    Ok(())
}
