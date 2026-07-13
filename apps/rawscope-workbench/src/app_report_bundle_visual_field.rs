//! Canonical generic visual-field report adapter.
//!
//! This is intentionally separate from the legacy scatter/timeline writers.
//! It accepts an already validated evidence snapshot and does not infer
//! analytical semantics from renderer state.  Full v2 bundle transaction
//! migration remains owned by the evidence/bundle remediation work.

use std::{error::Error, fs};

use rawscope_evidence::{
    visual_field_evidence_v1_json, VisualFieldEvidenceV1, VISUAL_FIELD_EVIDENCE_V1_ARTIFACT_KIND,
    VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION,
};
use serde::Serialize;

use crate::app_report_bundle::EvidenceReportBundlePaths;

const VISUAL_FIELD_REPORT_BUNDLE_ARTIFACT_KIND: &str = "visual-field-evidence-report-bundle";

impl EvidenceReportBundlePaths {
    pub(crate) fn write_visual_field(
        &self,
        evidence: &VisualFieldEvidenceV1,
    ) -> Result<(), Box<dyn Error>> {
        evidence.validate()?;
        self.transactional(|staged| staged.write_visual_field_unstaged(evidence))
    }

    fn write_visual_field_unstaged(
        &self,
        evidence: &VisualFieldEvidenceV1,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.bundle_dir)?;
        fs::write(
            &self.evidence_json_path,
            visual_field_evidence_v1_json(evidence)?,
        )?;
        fs::write(
            &self.evidence_markdown_path,
            visual_field_markdown(evidence),
        )?;
        fs::write(
            &self.visual_context_path,
            "RawScope visual context is represented by the canonical visual-field evidence artifact.\n",
        )?;
        let manifest = VisualFieldEvidenceReportBundleManifestRecord {
            artifact_kind: VISUAL_FIELD_REPORT_BUNDLE_ARTIFACT_KIND,
            bundle_schema_version: crate::app_report_bundle::EVIDENCE_REPORT_BUNDLE_SCHEMA_VERSION,
            evidence_artifact_kind: VISUAL_FIELD_EVIDENCE_V1_ARTIFACT_KIND,
            evidence_schema_version: VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION,
            evidence_json_path: &self.evidence_json_path,
            evidence_markdown_path: &self.evidence_markdown_path,
            visual_context_path: &self.visual_context_path,
            selected_row_count: evidence.selection.selected_row_count,
            dataset_row_count: evidence.source.row_count,
            field_generation: evidence.field.field_generation,
            export_timestamp_unix_ms: self.export_timestamp_unix_ms,
            export_counter: self.export_counter,
        };
        self.write_manifest_record(&manifest)?;
        Ok(())
    }
}

#[derive(Serialize)]
struct VisualFieldEvidenceReportBundleManifestRecord<'a> {
    artifact_kind: &'static str,
    bundle_schema_version: u32,
    evidence_artifact_kind: &'static str,
    evidence_schema_version: u32,
    evidence_json_path: &'a std::path::Path,
    evidence_markdown_path: &'a std::path::Path,
    visual_context_path: &'a std::path::Path,
    selected_row_count: u64,
    dataset_row_count: u64,
    field_generation: u64,
    export_timestamp_unix_ms: u128,
    export_counter: u64,
}

fn visual_field_markdown(evidence: &VisualFieldEvidenceV1) -> String {
    format!(
        "# RawScope visual-field evidence\n\n- schema: `{}` v{}\n- source: `{}`\n- rows: {}\n- selected rows: {}\n- field generation: {}\n- mapping: `{}`\n- quality: `{:?}`\n- freshness: `{:?}`\n",
        evidence.schema,
        evidence.schema_version,
        evidence.source.source_label,
        evidence.source.row_count,
        evidence.selection.selected_row_count,
        evidence.field.field_generation,
        evidence.field.mapping_identity,
        evidence.field.quality,
        evidence.field.freshness,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::{cohort::CohortGenerationCounter, selection::SelectionSnapshot};
    use rawscope_core::{RowId, SelectionId};
    use rawscope_data::{DatasetGenerationCounter, DatasetIdentity};
    use rawscope_evidence::{
        DensityModeEvidenceV1, EvidenceContext, EvidenceDocument, EvidenceVisualContext,
        ExactFieldEvidenceV1, FieldDomainEvidenceV1, FieldEvidenceV1, FieldKindEvidenceV1,
        FreshnessEvidenceV1, QualityTierEvidenceV1, RowCountAggregationEvidenceV1,
        VisualFieldModeEvidenceV1, VisualFieldProjectionEvidenceV1,
        VisualFieldProvenanceEvidenceV1,
    };
    use std::fs;

    fn sample_evidence() -> VisualFieldEvidenceV1 {
        let dataset_generation = DatasetGenerationCounter::default().mint();
        let cohort_generation = CohortGenerationCounter::default().mint();
        let snapshot = SelectionSnapshot::from_parts(
            dataset_generation,
            cohort_generation,
            SelectionId(1),
            [RowId(1), RowId(2)],
            [0],
            2,
        )
        .unwrap();
        let document = EvidenceDocument::from_selection(
            &snapshot,
            EvidenceContext {
                dataset_generation,
                cohort_generation,
                dataset_identity: DatasetIdentity::synthetic_scatter(1, 2),
                cohort_included_row_count: 2,
                source_rows_available: true,
                visual: EvidenceVisualContext {
                    x_min: 0.0,
                    x_max: 1.0,
                    y_min: 0.0,
                    y_max: 1.0,
                    grid_width: 2,
                    grid_height: 2,
                },
            },
        )
        .unwrap();
        let field = |id: u32, name: &str| FieldEvidenceV1 {
            column_id: id,
            display_name: name.into(),
            kind: FieldKindEvidenceV1::Numeric,
            domain: FieldDomainEvidenceV1::Numeric { min: 0.0, max: 1.0 },
            missing_count: 0,
            invalid_count: 0,
            projected_count: 2,
            time_quantization: None,
        };
        VisualFieldEvidenceV1::from_document(
            &document,
            VisualFieldProjectionEvidenceV1::NumericPair {
                x: field(0, "x"),
                y: field(1, "y"),
            },
            None,
            RowCountAggregationEvidenceV1 {
                eligible_row_count: 2,
                counted_row_count: 2,
                missing_row_count: 0,
                invalid_row_count: 0,
                weighted: false,
            },
            ExactFieldEvidenceV1 {
                field_generation: 1,
                mapping_identity: "numeric-pair:0:1".into(),
                dataset_generation: dataset_generation.get(),
                cohort_generation: cohort_generation.get(),
                width: 2,
                height: 2,
                x_domain: rawscope_evidence::NumericDomainEvidenceV1 { min: 0.0, max: 1.0 },
                y_domain: rawscope_evidence::NumericDomainEvidenceV1 { min: 0.0, max: 1.0 },
                bin_rule: "floor".into(),
                quality: QualityTierEvidenceV1::Exact,
                freshness: FreshnessEvidenceV1::Settled,
            },
            VisualFieldModeEvidenceV1::Density(DensityModeEvidenceV1 {
                transform: "linear".into(),
                normalization: "viewport_max".into(),
                palette: "neutral".into(),
                contours: Vec::new(),
                marginal_x_totals: vec![1, 1],
                marginal_y_totals: vec![1, 1],
                semantic_point_sample: rawscope_evidence::SemanticPointSampleEvidenceV1 {
                    eligible_count: 2,
                    rendered_count: 2,
                    sample_complete: true,
                },
            }),
            None,
            VisualFieldProvenanceEvidenceV1 {
                mapping_identity: "numeric-pair:0:1".into(),
                field_generation: 1,
                source: "test".into(),
                construction: "settled".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn generic_report_bundle_uses_canonical_visual_field_artifact() {
        let root =
            std::env::temp_dir().join(format!("rawscope-visual-field-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = EvidenceReportBundlePaths::next_available(&root, "report-visual-field", 1, 0);
        paths.write_visual_field(&sample_evidence()).unwrap();
        let manifest = fs::read_to_string(&paths.manifest_path).unwrap();
        assert!(manifest.contains(VISUAL_FIELD_EVIDENCE_V1_ARTIFACT_KIND));
        assert!(fs::read_to_string(&paths.evidence_json_path)
            .unwrap()
            .contains("rawscope.visual-field-evidence"));
        let _ = fs::remove_dir_all(&root);
    }
}
