//! Deterministic JSON and Markdown artifacts for scatter evidence v4.

use rawscope_data::{DatasetFilter, ScatterProjection};

use crate::{ScatterDensityMode, ScatterSelectionEvidenceV4, ScatterSelectionExportError};

#[path = "scatter_selection_export_v4_artifact.rs"]
mod artifact;

pub const SCATTER_SELECTION_EVIDENCE_V4_ARTIFACT_KIND: &str =
    "rawscope.scatter-selection-evidence.v4";

pub fn scatter_selection_evidence_v4_json(
    evidence: &ScatterSelectionEvidenceV4,
) -> Result<String, ScatterSelectionExportError> {
    artifact::to_json(evidence)
}

pub fn scatter_selection_evidence_v4_markdown(evidence: &ScatterSelectionEvidenceV4) -> String {
    let query = &evidence.visual_query;
    let mut markdown = String::from("# RawScope Scatter Selection Evidence v4\n\n");
    markdown
        .push_str("> Reproducible visual query with explicit cohort and bounded row evidence.\n\n");
    markdown.push_str("## Visual Query\n\n");
    markdown.push_str(&format!(
        "- Projection: {}\n- Projection sign: {}\n- Density mode: {}\n- Presentation: {}\n- Encoding: {} | {} | {}\n- Grid: {}x{}\n- X range: {:.6}..{:.6}\n- Y range: {:.6}..{:.6}\n\n",
        projection_variant(query.projection),
        projection_sign_label(query.projection),
        query.density_mode.label(),
        query.density_presentation.evidence_label(),
        query.density_encoding.transform.label(),
        query.density_encoding.palette.label(),
        query.density_encoding.normalization.label(),
        query.grid_width,
        query.grid_height,
        query.x_range.min,
        query.x_range.max,
        query.y_range.min,
        query.y_range.max,
    ));
    markdown.push_str("## Cohort\n\n");
    markdown.push_str(&format!(
        "- Full rows: {}\n- Included rows: {}\n- Excluded rows: {}\n- Active filters: {}\n\n",
        evidence.cohort.full_row_count,
        evidence.cohort.included_row_count,
        evidence.cohort.excluded_row_count,
        query.filters.len(),
    ));
    markdown.push_str("## Filters\n\n");
    if query.filters.is_empty() {
        markdown.push_str("_No active filters._\n\n");
    } else {
        for filter in &query.filters {
            markdown.push_str(&format!("- {}\n", filter_markdown(filter)));
        }
        markdown.push('\n');
    }
    if let Some(difference) = &query.difference {
        markdown.push_str("## Difference Density\n\n");
        markdown.push_str(&format!(
            "- Formula: `{}`\n- Baseline: `{}`\n- Baseline rows: {}\n- Active rows: {}\n- Maximum absolute share delta: {:.9}\n\n",
            difference.formula,
            difference.baseline,
            difference.baseline_total,
            difference.active_total,
            difference.max_abs_delta,
        ));
    }
    markdown.push_str("## Point Reveal\n\n");
    markdown.push_str(&format!(
        "- Mode: {}\n- Eligible points: {}\n- Rendered points: {}\n- Sampled: {}\n\n",
        point_reveal_mode(query.point_reveal.mode),
        query.point_reveal.eligible_count,
        query.point_reveal.rendered_count,
        query.point_reveal.sampled,
    ));
    if let Some(relief) = query.relief {
        markdown.push_str("## Relief Shading\n\n");
        markdown.push_str(&format!(
            "- Height strength: {:.3}\n- Normal radius: {} bins\n- Light azimuth: {:.3} degrees\n- Light elevation: {:.3} degrees\n- Ambient: {:.3}\n- Horizon shadow: {:.3}\n- Contours: {:.3}\n\n",
            relief.height_strength,
            relief.normal_radius_bins,
            relief.light_azimuth_degrees,
            relief.light_elevation_degrees,
            relief.ambient_strength,
            relief.shadow_strength,
            relief.contour_strength,
        ));
    }
    if let Some(pin) = &evidence.pinned_inspection {
        markdown.push_str("## Pinned Inspection\n\n");
        markdown.push_str(&format!(
            "- Bin: ({}, {})\n- Rows: {}\n- Row-id sample: {} of at most {}\n\n",
            pin.bin_x,
            pin.bin_y,
            pin.row_count,
            pin.row_id_sample.len(),
            pin.sample_limit,
        ));
    }
    markdown.push_str("## Selection\n\n");
    markdown.push_str(&format!(
        "- Selected rows: {}\n- Selected percentage: {:.4}%\n- Row-id sample size: {}\n- Source-row sample size: {}\n",
        evidence.selected_row_count,
        evidence.selected_percentage,
        evidence.selected_row_id_sample.len(),
        evidence.selected_source_row_sample.len(),
    ));
    markdown
}

pub(super) fn projection_variant(value: ScatterProjection) -> &'static str {
    match value {
        ScatterProjection::RawXY => "raw_xy",
        ScatterProjection::MeanDifference => "mean_difference",
    }
}

pub(super) fn projection_sign_label(value: ScatterProjection) -> &'static str {
    match value {
        ScatterProjection::RawXY => "not_applicable",
        ScatterProjection::MeanDifference => "difference_is_first_value_minus_second_value",
    }
}

pub(super) fn density_mode(value: ScatterDensityMode) -> &'static str {
    match value {
        ScatterDensityMode::AbsoluteDensity => "absolute_density",
        ScatterDensityMode::FilteredDifference => "filtered_difference",
    }
}

pub(super) fn point_reveal_mode(value: crate::PointRevealMode) -> &'static str {
    match value {
        crate::PointRevealMode::Off => "off",
        crate::PointRevealMode::Auto => "auto",
    }
}

fn filter_markdown(filter: &DatasetFilter) -> String {
    match filter {
        DatasetFilter::NumericRange {
            column_name,
            min_inclusive,
            max_inclusive,
            include_missing,
        } => format!(
            "{column_name}: {min_inclusive}..={max_inclusive}; include missing: {include_missing}"
        ),
        DatasetFilter::Categories {
            column_name,
            included_values,
            include_missing,
        } => {
            let mut values = included_values.clone();
            values.sort();
            values.dedup();
            format!(
                "{column_name}: [{}]; include missing: {include_missing}",
                values.join(", ")
            )
        }
    }
}
