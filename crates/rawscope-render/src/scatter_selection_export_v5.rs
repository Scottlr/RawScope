//! Deterministic JSON and Markdown artifacts for scatter evidence v5.

use serde_json::Value;

use crate::{
    scatter_selection_evidence_v4_json, DifferenceDirectionEvidenceV5,
    PinnedDifferenceInspectionEvidenceV5, ScatterSelectionEvidenceV5, ScatterSelectionExportError,
};

#[path = "scatter_selection_export_v5_artifact.rs"]
mod artifact;

pub const SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND: &str =
    "rawscope.scatter-selection-evidence.v5";

pub fn scatter_selection_evidence_v5_json(
    evidence: &ScatterSelectionEvidenceV5,
) -> Result<String, ScatterSelectionExportError> {
    let mut payload: Value =
        serde_json::from_str(&scatter_selection_evidence_v4_json_from_v5(evidence)?)?;
    let object = payload
        .as_object_mut()
        .expect("v4 scatter evidence serializer returns an object");
    object.insert(
        "artifact_kind".to_string(),
        Value::String(SCATTER_SELECTION_EVIDENCE_V5_ARTIFACT_KIND.to_string()),
    );
    object.insert(
        "schema_version".to_string(),
        Value::from(evidence.schema_version),
    );
    if let Some(context) = &evidence.session_context {
        object.insert(
            "session_context".to_string(),
            serde_json::to_value(artifact::SessionContextArtifact::from(context))?,
        );
    }
    object.remove("pinned_inspection");
    if let Some(pin) = &evidence.pinned_inspection {
        object.insert(
            "pinned_inspection".to_string(),
            serde_json::to_value(artifact::PinnedInspectionArtifact::from(pin))?,
        );
    }
    serde_json::to_string_pretty(&payload)
}

fn scatter_selection_evidence_v4_json_from_v5(
    evidence: &ScatterSelectionEvidenceV5,
) -> Result<String, ScatterSelectionExportError> {
    let legacy = crate::ScatterSelectionEvidenceV4 {
        schema_version: 4,
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
        pinned_inspection: None,
    };
    scatter_selection_evidence_v4_json(&legacy)
}

pub fn scatter_selection_evidence_v5_markdown(evidence: &ScatterSelectionEvidenceV5) -> String {
    let mut markdown =
        crate::scatter_selection_evidence_v4_markdown(&crate::ScatterSelectionEvidenceV4 {
            schema_version: 4,
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
            pinned_inspection: None,
        })
        .replacen("Evidence v4", "Evidence v5", 1);
    markdown.push_str("\n## Session Context\n\n");
    if let Some(context) = &evidence.session_context {
        markdown.push_str(&format!(
            "- Session artifact: `{}` v{}\n- Data format: {}\n- Display name: {}\n- Evidence-key column: {}\n\n",
            context.session_artifact_kind,
            context.session_schema_version,
            context.data_format.label(),
            context.display_name.as_deref().unwrap_or("not provided"),
            context.evidence_key_column.as_deref().unwrap_or("not provided"),
        ));
    } else {
        markdown.push_str(
            "_No session context; this evidence came from direct or synthetic startup._\n\n",
        );
    }
    if let Some(pin) = &evidence.pinned_inspection {
        markdown.push_str("## Pinned Inspection Context\n\n");
        markdown.push_str(&format!(
            "- Exact bin: ({}, {})\n- X range: {:.6}..{:.6}\n- Y range: {:.6}..{:.6}\n- Exact rows: {} ({:.4}% of active cohort)\n- Occupied-cell density percentile: {}\n- Neighborhood: radius {} bins, {} rows ({:.4}% of active cohort)\n- Row-id sample: {} of at most {}\n",
            pin.bin_x,
            pin.bin_y,
            pin.x_range.min,
            pin.x_range.max,
            pin.y_range.min,
            pin.y_range.max,
            pin.row_count,
            pin.active_share * 100.0,
            format_optional_percentile(pin.occupied_density_percentile),
            pin.neighborhood_radius_bins,
            pin.neighborhood_row_count,
            pin.neighborhood_share * 100.0,
            pin.row_id_sample.len(),
            pin.sample_limit,
        ));
        if let Some(difference) = &pin.difference {
            markdown.push_str(&format_difference_markdown(difference));
        }
        markdown.push_str(&format!(
            "- Natural-key values: {} of at most {} sampled values\n\n",
            pin.evidence_key_values.len(),
            pin.sample_limit,
        ));
    }
    markdown
}

fn format_optional_percentile(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.2}%", value * 100.0))
        .unwrap_or_else(|| "not available".to_string())
}

fn format_difference_markdown(value: &PinnedDifferenceInspectionEvidenceV5) -> String {
    format!(
        "- Difference: {} ({:+.4} percentage points); strength percentile: {}\n",
        difference_direction_label(value.direction),
        value.share_delta * 100.0,
        format_optional_percentile(value.absolute_delta_percentile),
    )
}

fn difference_direction_label(value: DifferenceDirectionEvidenceV5) -> &'static str {
    match value {
        DifferenceDirectionEvidenceV5::MoreCommonInActive => "more common in active cohort",
        DifferenceDirectionEvidenceV5::LessCommonInActive => "less common in active cohort",
        DifferenceDirectionEvidenceV5::Unchanged => "unchanged",
    }
}
