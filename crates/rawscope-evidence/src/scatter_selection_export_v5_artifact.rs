//! Private serde DTOs for v5-only pinned and session context.

use rawscope_core::F32Range;
use serde::Serialize;

use crate::{
    DifferenceDirectionEvidenceV5, PinnedScatterInspectionEvidenceV5, SessionDataFormatEvidenceV5,
    SessionEvidenceContextV5,
};

#[derive(Serialize)]
pub(super) struct SessionContextArtifact<'a> {
    session_artifact_kind: &'static str,
    session_schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<&'a str>,
    data_format: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence_key_column: Option<&'a str>,
}

impl<'a> From<&'a SessionEvidenceContextV5> for SessionContextArtifact<'a> {
    fn from(value: &'a SessionEvidenceContextV5) -> Self {
        Self {
            session_artifact_kind: value.session_artifact_kind,
            session_schema_version: value.session_schema_version,
            display_name: value.display_name.as_deref(),
            data_format: match value.data_format {
                SessionDataFormatEvidenceV5::Csv => "csv",
                SessionDataFormatEvidenceV5::Parquet => "parquet",
            },
            evidence_key_column: value.evidence_key_column.as_deref(),
        }
    }
}

#[derive(Serialize)]
pub(super) struct PinnedInspectionArtifact {
    bin_x: u32,
    bin_y: u32,
    x_range: RangeArtifact,
    y_range: RangeArtifact,
    row_count: u32,
    active_share: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupied_density_percentile: Option<f64>,
    neighborhood_radius_bins: u32,
    neighborhood_row_count: u64,
    neighborhood_share: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    difference: Option<DifferenceArtifact>,
    row_id_sample: Vec<u64>,
    sample_limit: usize,
    evidence_key_values: Vec<String>,
}

impl From<&PinnedScatterInspectionEvidenceV5> for PinnedInspectionArtifact {
    fn from(value: &PinnedScatterInspectionEvidenceV5) -> Self {
        Self {
            bin_x: value.bin_x,
            bin_y: value.bin_y,
            x_range: value.x_range.into(),
            y_range: value.y_range.into(),
            row_count: value.row_count,
            active_share: value.active_share,
            occupied_density_percentile: value.occupied_density_percentile,
            neighborhood_radius_bins: value.neighborhood_radius_bins,
            neighborhood_row_count: value.neighborhood_row_count,
            neighborhood_share: value.neighborhood_share,
            difference: value.difference.as_ref().map(DifferenceArtifact::from),
            row_id_sample: value.row_id_sample.iter().map(|row_id| row_id.0).collect(),
            sample_limit: value.sample_limit,
            evidence_key_values: value.evidence_key_values.clone(),
        }
    }
}

#[derive(Serialize)]
struct RangeArtifact {
    min: f32,
    max: f32,
}

impl From<F32Range> for RangeArtifact {
    fn from(value: F32Range) -> Self {
        Self {
            min: value.min,
            max: value.max,
        }
    }
}

#[derive(Serialize)]
struct DifferenceArtifact {
    baseline_count: u32,
    active_count: u32,
    baseline_share: f64,
    active_share: f64,
    share_delta: f64,
    direction: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    absolute_delta_percentile: Option<f64>,
    formula: &'static str,
    baseline: &'static str,
    baseline_total: u64,
    active_total: u64,
}

impl From<&crate::PinnedDifferenceInspectionEvidenceV5> for DifferenceArtifact {
    fn from(value: &crate::PinnedDifferenceInspectionEvidenceV5) -> Self {
        Self {
            baseline_count: value.baseline_count,
            active_count: value.active_count,
            baseline_share: value.baseline_share,
            active_share: value.active_share,
            share_delta: value.share_delta,
            direction: match value.direction {
                DifferenceDirectionEvidenceV5::MoreCommonInActive => "more_common_in_active",
                DifferenceDirectionEvidenceV5::LessCommonInActive => "less_common_in_active",
                DifferenceDirectionEvidenceV5::Unchanged => "unchanged",
            },
            absolute_delta_percentile: value.absolute_delta_percentile,
            formula: value.formula,
            baseline: value.baseline,
            baseline_total: value.baseline_total,
            active_total: value.active_total,
        }
    }
}
