//! Private serde DTO mapping for scatter evidence v4.

use rawscope_core::F32Range;
use rawscope_data::DatasetFilter;
use serde::Serialize;
use serde_json::Value;

use super::{density_mode, point_reveal_mode, projection_sign_label, projection_variant};
use crate::{
    scatter_selection_evidence_v3_json, DifferenceDensityEvidenceConfig,
    PinnedScatterInspectionEvidence, PointRevealEvidence, ReliefFieldConfig,
    ScatterSelectionEvidenceV3, ScatterSelectionEvidenceV4, ScatterSelectionExportError,
    ScatterVisualQueryV4, SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND,
};

pub(super) fn to_json(
    evidence: &ScatterSelectionEvidenceV4,
) -> Result<String, ScatterSelectionExportError> {
    evidence.validate().map_err(|error| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })?;
    serde_json::to_string_pretty(&ScatterSelectionEvidenceV4Artifact::from(evidence))
}

#[derive(Serialize)]
struct ScatterSelectionEvidenceV4Artifact {
    artifact_kind: &'static str,
    schema_version: u32,
    dataset_identity: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_dataset_profile: Option<String>,
    visual_query: ScatterVisualQueryV4Artifact,
    cohort: ScatterCohortArtifact,
    selected_row_count: Value,
    selected_percentage: Value,
    row_id_sample: Value,
    selected_record_sample: Value,
    source_columns: Value,
    selected_source_row_sample: Value,
    comparison: Value,
    aggregate_context: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinned_inspection: Option<PinnedInspectionArtifact>,
}

impl From<&ScatterSelectionEvidenceV4> for ScatterSelectionEvidenceV4Artifact {
    fn from(evidence: &ScatterSelectionEvidenceV4) -> Self {
        let legacy = legacy_v3_payload(evidence);
        Self {
            artifact_kind: SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND,
            schema_version: evidence.schema_version,
            dataset_identity: legacy["dataset_identity"].clone(),
            active_dataset_profile: evidence
                .active_dataset_profile
                .map(|profile| profile.as_str().to_string()),
            visual_query: ScatterVisualQueryV4Artifact::from(&evidence.visual_query),
            cohort: ScatterCohortArtifact {
                full_row_count: evidence.cohort.full_row_count,
                included_row_count: evidence.cohort.included_row_count,
                excluded_row_count: evidence.cohort.excluded_row_count,
            },
            selected_row_count: legacy["selected_row_count"].clone(),
            selected_percentage: legacy["selected_percentage"].clone(),
            row_id_sample: legacy["row_id_sample"].clone(),
            selected_record_sample: legacy["selected_record_sample"].clone(),
            source_columns: legacy["source_columns"].clone(),
            selected_source_row_sample: legacy["selected_source_row_sample"].clone(),
            comparison: legacy["comparison"].clone(),
            aggregate_context: legacy["aggregate_context"].clone(),
            pinned_inspection: evidence
                .pinned_inspection
                .as_ref()
                .map(PinnedInspectionArtifact::from),
        }
    }
}

fn legacy_v3_payload(evidence: &ScatterSelectionEvidenceV4) -> Value {
    let legacy = ScatterSelectionEvidenceV3 {
        schema_version: 3,
        dataset_identity: evidence.dataset_identity.clone(),
        active_dataset_profile: evidence.active_dataset_profile,
        view: crate::ScatterEvidenceViewV3 {
            x_range: evidence.visual_query.x_range,
            y_range: evidence.visual_query.y_range,
            grid_width: evidence.visual_query.grid_width,
            grid_height: evidence.visual_query.grid_height,
            density_encoding: evidence.visual_query.density_encoding,
            density_presentation: evidence.visual_query.density_presentation,
        },
        selected_row_count: evidence.selected_row_count,
        selected_percentage: evidence.selected_percentage,
        selected_row_id_sample: evidence.selected_row_id_sample.clone(),
        selected_record_sample: evidence.selected_record_sample.clone(),
        selected_source_column_names: evidence.selected_source_column_names.clone(),
        selected_source_row_sample: evidence.selected_source_row_sample.clone(),
        comparison: evidence.comparison.clone(),
        aggregate_context: evidence.aggregate_context.clone(),
    };
    serde_json::from_str(
        &scatter_selection_evidence_v3_json(&legacy).expect("v3 DTO is serializable"),
    )
    .expect("v3 JSON is valid")
}

#[derive(Serialize)]
struct ScatterVisualQueryV4Artifact {
    x_range: RangeArtifact,
    y_range: RangeArtifact,
    grid_width: u32,
    grid_height: u32,
    projection: ProjectionArtifact,
    filters: Vec<DatasetFilterArtifact>,
    density_mode: &'static str,
    density_encoding: crate::DensityEncoding,
    density_presentation: crate::ScatterDensityPresentation,
    #[serde(skip_serializing_if = "Option::is_none")]
    difference: Option<DifferenceArtifact>,
    point_reveal: PointRevealArtifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    relief: Option<ReliefArtifact>,
}

impl From<&ScatterVisualQueryV4> for ScatterVisualQueryV4Artifact {
    fn from(query: &ScatterVisualQueryV4) -> Self {
        Self {
            x_range: query.x_range.into(),
            y_range: query.y_range.into(),
            grid_width: query.grid_width,
            grid_height: query.grid_height,
            projection: ProjectionArtifact {
                variant: projection_variant(query.projection),
                mean_difference_sign: projection_sign_label(query.projection),
            },
            filters: query
                .filters
                .iter()
                .map(DatasetFilterArtifact::from)
                .collect(),
            density_mode: density_mode(query.density_mode),
            density_encoding: query.density_encoding,
            density_presentation: query.density_presentation,
            difference: query.difference.as_ref().map(DifferenceArtifact::from),
            point_reveal: PointRevealArtifact::from(&query.point_reveal),
            relief: query.relief.map(ReliefArtifact::from),
        }
    }
}

#[derive(Serialize)]
struct RangeArtifact {
    min: f32,
    max: f32,
}

impl From<F32Range> for RangeArtifact {
    fn from(range: F32Range) -> Self {
        Self {
            min: range.min,
            max: range.max,
        }
    }
}

#[derive(Serialize)]
struct ProjectionArtifact {
    variant: &'static str,
    mean_difference_sign: &'static str,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DatasetFilterArtifact {
    NumericRange {
        column_name: String,
        min_inclusive: f64,
        max_inclusive: f64,
        include_missing: bool,
    },
    Categories {
        column_name: String,
        included_values: Vec<String>,
        include_missing: bool,
    },
}

impl From<&DatasetFilter> for DatasetFilterArtifact {
    fn from(filter: &DatasetFilter) -> Self {
        match filter {
            DatasetFilter::NumericRange {
                column_name,
                min_inclusive,
                max_inclusive,
                include_missing,
            } => Self::NumericRange {
                column_name: column_name.clone(),
                min_inclusive: *min_inclusive,
                max_inclusive: *max_inclusive,
                include_missing: *include_missing,
            },
            DatasetFilter::Categories {
                column_name,
                included_values,
                include_missing,
            } => {
                let mut included_values = included_values.clone();
                included_values.sort();
                included_values.dedup();
                Self::Categories {
                    column_name: column_name.clone(),
                    included_values,
                    include_missing: *include_missing,
                }
            }
        }
    }
}

#[derive(Serialize)]
struct ScatterCohortArtifact {
    full_row_count: usize,
    included_row_count: usize,
    excluded_row_count: usize,
}

#[derive(Serialize)]
struct DifferenceArtifact {
    formula: &'static str,
    baseline: &'static str,
    baseline_total: u64,
    active_total: u64,
    max_abs_delta: f32,
}

impl From<&DifferenceDensityEvidenceConfig> for DifferenceArtifact {
    fn from(value: &DifferenceDensityEvidenceConfig) -> Self {
        Self {
            formula: value.formula,
            baseline: value.baseline,
            baseline_total: value.baseline_total,
            active_total: value.active_total,
            max_abs_delta: value.max_abs_delta,
        }
    }
}

#[derive(Serialize)]
struct PointRevealArtifact {
    mode: &'static str,
    eligible_count: usize,
    rendered_count: usize,
    sampled: bool,
}

impl From<&PointRevealEvidence> for PointRevealArtifact {
    fn from(value: &PointRevealEvidence) -> Self {
        Self {
            mode: point_reveal_mode(value.mode),
            eligible_count: value.eligible_count,
            rendered_count: value.rendered_count,
            sampled: value.sampled,
        }
    }
}

#[derive(Serialize)]
struct ReliefArtifact {
    height_strength: f32,
    normal_radius_bins: u32,
    light_azimuth_degrees: f32,
    light_elevation_degrees: f32,
    ambient_strength: f32,
    shadow_strength: f32,
    contour_strength: f32,
}

impl From<ReliefFieldConfig> for ReliefArtifact {
    fn from(value: ReliefFieldConfig) -> Self {
        Self {
            height_strength: value.height_strength,
            normal_radius_bins: value.normal_radius_bins,
            light_azimuth_degrees: value.light_azimuth_degrees,
            light_elevation_degrees: value.light_elevation_degrees,
            ambient_strength: value.ambient_strength,
            shadow_strength: value.shadow_strength,
            contour_strength: value.contour_strength,
        }
    }
}

#[derive(Serialize)]
struct PinnedInspectionArtifact {
    bin_x: u32,
    bin_y: u32,
    x_range: RangeArtifact,
    y_range: RangeArtifact,
    row_count: u32,
    row_id_sample: Vec<u64>,
    sample_limit: usize,
}

impl From<&PinnedScatterInspectionEvidence> for PinnedInspectionArtifact {
    fn from(value: &PinnedScatterInspectionEvidence) -> Self {
        Self {
            bin_x: value.bin_x,
            bin_y: value.bin_y,
            x_range: value.x_range.into(),
            y_range: value.y_range.into(),
            row_count: value.row_count,
            row_id_sample: value.row_id_sample.iter().map(|id| id.0).collect(),
            sample_limit: value.sample_limit,
        }
    }
}
