//! Additive scatter evidence v5 with local session and pinned inspection context.

use std::{error::Error, fmt};

use rawscope_core::{F32Range, RowId};
use rawscope_data::DatasetIdentity;

use crate::{
    ScatterDensityMode, ScatterSelectionEvidenceV4, ScatterVisualQueryV4, DIFFERENCE_BASELINE_ID,
    DIFFERENCE_FORMULA_ID, INSPECTION_NEIGHBORHOOD_RADIUS_BINS,
};

pub const SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION: u32 = 5;
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionEvidenceV5 {
    pub schema_version: u32,
    pub dataset_identity: DatasetIdentity,
    pub active_dataset_profile: Option<rawscope_data::DatasetProfileId>,
    pub visual_query: ScatterVisualQueryV4,
    pub cohort: crate::ScatterCohortEvidence,
    pub selected_row_count: usize,
    pub selected_percentage: f32,
    pub selected_row_id_sample: Vec<RowId>,
    pub selected_record_sample: Vec<crate::SelectedPointSampleV2>,
    pub selected_source_column_names: Vec<String>,
    pub selected_source_row_sample: Vec<crate::SelectedSourceRowSample>,
    pub comparison: crate::ScatterSelectionComparison,
    pub aggregate_context: crate::ScatterAggregateEvidenceContext,
    pub session_context: Option<SessionEvidenceContextV5>,
    pub pinned_inspection: Option<PinnedScatterInspectionEvidenceV5>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDataFormatEvidenceV5 {
    Csv,
    Parquet,
}

impl SessionDataFormatEvidenceV5 {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Parquet => "parquet",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEvidenceContextV5 {
    pub session_artifact_kind: &'static str,
    pub session_schema_version: u32,
    pub display_name: Option<String>,
    pub data_format: SessionDataFormatEvidenceV5,
    pub evidence_key_column: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PinnedScatterInspectionEvidenceV5 {
    pub bin_x: u32,
    pub bin_y: u32,
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub row_count: u32,
    pub active_share: f64,
    pub occupied_density_percentile: Option<f64>,
    pub neighborhood_radius_bins: u32,
    pub neighborhood_row_count: u64,
    pub neighborhood_share: f64,
    pub difference: Option<PinnedDifferenceInspectionEvidenceV5>,
    pub row_id_sample: Vec<RowId>,
    pub sample_limit: usize,
    pub evidence_key_values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceDirectionEvidenceV5 {
    MoreCommonInActive,
    LessCommonInActive,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PinnedDifferenceInspectionEvidenceV5 {
    pub baseline_count: u32,
    pub active_count: u32,
    pub baseline_share: f64,
    pub active_share: f64,
    pub share_delta: f64,
    pub direction: DifferenceDirectionEvidenceV5,
    pub absolute_delta_percentile: Option<f64>,
    pub formula: &'static str,
    pub baseline: &'static str,
    pub baseline_total: u64,
    pub active_total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterSelectionEvidenceV5Error {
    InvalidSchemaVersion,
    LegacySchemaVersion,
    InvalidSessionContext,
    InvalidShare,
    InvalidPercentile,
    InvalidNeighborhoodRadius,
    InvalidNeighborhood,
    InvalidSample,
    MissingEvidenceKeyColumn,
    DifferencePresenceMismatch,
    DifferenceContractMismatch,
    DifferenceMetricMismatch,
}

impl fmt::Display for ScatterSelectionEvidenceV5Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidSchemaVersion => "scatter evidence v5 schema version must be exactly 5",
            Self::LegacySchemaVersion => "scatter evidence v5 requires a v4 source contract",
            Self::InvalidSessionContext => "session evidence context is incoherent",
            Self::InvalidShare => "scatter evidence share must be finite and within 0..=1",
            Self::InvalidPercentile => {
                "scatter evidence percentile must be finite and within 0..=1"
            }
            Self::InvalidNeighborhoodRadius => "scatter evidence neighborhood radius must be one",
            Self::InvalidNeighborhood => "scatter evidence neighborhood is incoherent",
            Self::InvalidSample => "scatter evidence bounded sample is incoherent",
            Self::MissingEvidenceKeyColumn => {
                "evidence key values require a session evidence-key column"
            }
            Self::DifferencePresenceMismatch => {
                "pinned difference context must match the visual density mode"
            }
            Self::DifferenceContractMismatch => {
                "pinned difference context formula or totals are incoherent"
            }
            Self::DifferenceMetricMismatch => {
                "pinned difference shares, delta, or direction are incoherent"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for ScatterSelectionEvidenceV5Error {}

impl ScatterSelectionEvidenceV5 {
    pub fn from_v4(
        evidence: &ScatterSelectionEvidenceV4,
        session_context: Option<SessionEvidenceContextV5>,
        pinned_inspection: Option<PinnedScatterInspectionEvidenceV5>,
    ) -> Result<Self, ScatterSelectionEvidenceV5Error> {
        let result = Self {
            schema_version: SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION,
            dataset_identity: evidence.dataset_identity.clone(),
            active_dataset_profile: evidence.active_dataset_profile,
            visual_query: evidence.visual_query.clone(),
            cohort: evidence.cohort.clone(),
            selected_row_count: evidence.selected_row_count,
            selected_percentage: evidence.selected_percentage,
            selected_row_id_sample: evidence.selected_row_id_sample.clone(),
            selected_record_sample: evidence.selected_record_sample.clone(),
            selected_source_column_names: evidence.selected_source_column_names.clone(),
            selected_source_row_sample: evidence.selected_source_row_sample.clone(),
            comparison: evidence.comparison.clone(),
            aggregate_context: evidence.aggregate_context.clone(),
            session_context,
            pinned_inspection,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), ScatterSelectionEvidenceV5Error> {
        if self.schema_version != SCATTER_SELECTION_EVIDENCE_V5_SCHEMA_VERSION {
            return Err(ScatterSelectionEvidenceV5Error::InvalidSchemaVersion);
        }
        if self.visual_query.density_mode == ScatterDensityMode::FilteredDifference
            && self.visual_query.difference.is_none()
        {
            return Err(ScatterSelectionEvidenceV5Error::DifferencePresenceMismatch);
        }
        if self.visual_query.density_mode == ScatterDensityMode::AbsoluteDensity
            && self
                .pinned_inspection
                .as_ref()
                .is_some_and(|pin| pin.difference.is_some())
        {
            return Err(ScatterSelectionEvidenceV5Error::DifferencePresenceMismatch);
        }
        if let Some(context) = &self.session_context {
            if context.session_artifact_kind != rawscope_session_artifact_kind()
                || context.session_schema_version != 1
            {
                return Err(ScatterSelectionEvidenceV5Error::InvalidSessionContext);
            }
        }
        if let Some(pin) = &self.pinned_inspection {
            validate_pin(self, pin)?;
        }
        Ok(())
    }
}

fn rawscope_session_artifact_kind() -> &'static str {
    "rawscope.session"
}

fn validate_pin(
    evidence: &ScatterSelectionEvidenceV5,
    pin: &PinnedScatterInspectionEvidenceV5,
) -> Result<(), ScatterSelectionEvidenceV5Error> {
    validate_unit_interval(pin.active_share)?;
    validate_optional_percentile(pin.occupied_density_percentile)?;
    validate_unit_interval(pin.neighborhood_share)?;
    if pin.neighborhood_radius_bins != INSPECTION_NEIGHBORHOOD_RADIUS_BINS {
        return Err(ScatterSelectionEvidenceV5Error::InvalidNeighborhoodRadius);
    }
    if pin.neighborhood_row_count < u64::from(pin.row_count)
        || pin.neighborhood_row_count > evidence.cohort.included_row_count as u64
        || !approximately_equal(
            pin.active_share,
            ratio(
                u64::from(pin.row_count),
                evidence.cohort.included_row_count as u64,
            ),
        )
        || !approximately_equal(
            pin.neighborhood_share,
            ratio(
                pin.neighborhood_row_count,
                evidence.cohort.included_row_count as u64,
            ),
        )
    {
        return Err(ScatterSelectionEvidenceV5Error::InvalidNeighborhood);
    }
    if pin.sample_limit == 0
        || pin.row_id_sample.len() > pin.sample_limit
        || pin.row_id_sample.len() > pin.row_count as usize
        || pin.evidence_key_values.len() > pin.sample_limit
        || pin.evidence_key_values.len() > pin.row_id_sample.len()
    {
        return Err(ScatterSelectionEvidenceV5Error::InvalidSample);
    }
    if !pin.evidence_key_values.is_empty()
        && evidence
            .session_context
            .as_ref()
            .and_then(|context| context.evidence_key_column.as_ref())
            .is_none()
    {
        return Err(ScatterSelectionEvidenceV5Error::MissingEvidenceKeyColumn);
    }
    if let Some(difference) = &pin.difference {
        if evidence.visual_query.density_mode != ScatterDensityMode::FilteredDifference {
            return Err(ScatterSelectionEvidenceV5Error::DifferencePresenceMismatch);
        }
        validate_difference(evidence, pin, difference)?;
    } else if evidence.visual_query.density_mode == ScatterDensityMode::FilteredDifference {
        return Err(ScatterSelectionEvidenceV5Error::DifferencePresenceMismatch);
    }
    Ok(())
}

fn validate_difference(
    evidence: &ScatterSelectionEvidenceV5,
    pin: &PinnedScatterInspectionEvidenceV5,
    difference: &PinnedDifferenceInspectionEvidenceV5,
) -> Result<(), ScatterSelectionEvidenceV5Error> {
    let Some(config) = evidence.visual_query.difference.as_ref() else {
        return Err(ScatterSelectionEvidenceV5Error::DifferencePresenceMismatch);
    };
    if difference.formula != DIFFERENCE_FORMULA_ID
        || difference.baseline != DIFFERENCE_BASELINE_ID
        || difference.formula != config.formula
        || difference.baseline != config.baseline
        || difference.baseline_total != config.baseline_total
        || difference.active_total != config.active_total
        || difference.baseline_total != evidence.cohort.full_row_count as u64
        || difference.active_total != evidence.cohort.included_row_count as u64
        || difference.active_count != pin.row_count
    {
        return Err(ScatterSelectionEvidenceV5Error::DifferenceContractMismatch);
    }
    validate_unit_interval(difference.baseline_share)?;
    validate_unit_interval(difference.active_share)?;
    validate_optional_percentile(difference.absolute_delta_percentile)?;
    let expected_baseline_share = ratio(
        u64::from(difference.baseline_count),
        difference.baseline_total,
    );
    let expected_active_share = ratio(u64::from(difference.active_count), difference.active_total);
    let expected_delta = expected_active_share - expected_baseline_share;
    let expected_direction = if expected_delta > 0.0 {
        DifferenceDirectionEvidenceV5::MoreCommonInActive
    } else if expected_delta < 0.0 {
        DifferenceDirectionEvidenceV5::LessCommonInActive
    } else {
        DifferenceDirectionEvidenceV5::Unchanged
    };
    if !approximately_equal(difference.baseline_share, expected_baseline_share)
        || !approximately_equal(difference.active_share, expected_active_share)
        || !approximately_equal(difference.share_delta, expected_delta)
        || difference.direction != expected_direction
    {
        return Err(ScatterSelectionEvidenceV5Error::DifferenceMetricMismatch);
    }
    Ok(())
}

fn validate_unit_interval(value: f64) -> Result<(), ScatterSelectionEvidenceV5Error> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScatterSelectionEvidenceV5Error::InvalidShare)
    }
}

fn validate_optional_percentile(value: Option<f64>) -> Result<(), ScatterSelectionEvidenceV5Error> {
    if value.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
        Err(ScatterSelectionEvidenceV5Error::InvalidPercentile)
    } else {
        Ok(())
    }
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-12
}

#[cfg(test)]
#[path = "scatter_selection_evidence_v5_tests.rs"]
mod tests;
