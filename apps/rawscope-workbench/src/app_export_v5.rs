//! Workbench projection for additive scatter evidence v5 context.

use rawscope_render::{
    DifferenceDirectionEvidenceV5, PinnedDifferenceInspectionEvidenceV5,
    PinnedScatterInspectionEvidenceV5, ScatterInspectionConfig, ScatterSelectionEvidenceV4,
    ScatterSelectionEvidenceV5, SessionDataFormatEvidenceV5, SessionEvidenceContextV5,
    DIFFERENCE_BASELINE_ID, DIFFERENCE_FORMULA_ID,
};

use crate::app::WorkbenchApp;

impl WorkbenchApp {
    pub(crate) fn scatter_selection_evidence_v5(
        &self,
        evidence_v4: &ScatterSelectionEvidenceV4,
    ) -> Option<ScatterSelectionEvidenceV5> {
        let session_context =
            self.workbench_state
                .active_session
                .as_ref()
                .map(|session| SessionEvidenceContextV5 {
                    session_artifact_kind: rawscope_session::RAWSCOPE_SESSION_ARTIFACT_KIND,
                    session_schema_version: rawscope_session::RAWSCOPE_SESSION_SCHEMA_VERSION,
                    display_name: session.display_name.clone(),
                    data_format: match session.data_format {
                        rawscope_session::SessionDataFormat::Csv => {
                            SessionDataFormatEvidenceV5::Csv
                        }
                        rawscope_session::SessionDataFormat::Parquet => {
                            SessionDataFormatEvidenceV5::Parquet
                        }
                    },
                    evidence_key_column: session
                        .evidence_key
                        .as_ref()
                        .map(|key| key.column_name.clone()),
                });
        let pinned_inspection = self.scatter_inspection.pinned.as_ref().map(|pinned| {
            PinnedScatterInspectionEvidenceV5 {
                bin_x: pinned.summary.hit.bin_x,
                bin_y: pinned.summary.hit.bin_y,
                x_range: pinned.summary.hit.x_range,
                y_range: pinned.summary.hit.y_range,
                row_count: pinned.summary.hit.count,
                active_share: pinned.summary.active_share,
                occupied_density_percentile: pinned.summary.occupied_density_percentile,
                neighborhood_radius_bins: pinned.summary.neighborhood_radius_bins,
                neighborhood_row_count: pinned.summary.neighborhood_count,
                neighborhood_share: pinned.summary.neighborhood_share,
                difference: pinned.difference.map(|difference| {
                    PinnedDifferenceInspectionEvidenceV5 {
                        baseline_count: difference.baseline_count,
                        active_count: difference.active_count,
                        baseline_share: difference.baseline_share,
                        active_share: difference.active_share,
                        share_delta: difference.share_delta,
                        direction: match difference.direction {
                            rawscope_render::DifferenceDirection::MoreCommonInActive => {
                                DifferenceDirectionEvidenceV5::MoreCommonInActive
                            }
                            rawscope_render::DifferenceDirection::LessCommonInActive => {
                                DifferenceDirectionEvidenceV5::LessCommonInActive
                            }
                            rawscope_render::DifferenceDirection::Unchanged => {
                                DifferenceDirectionEvidenceV5::Unchanged
                            }
                        },
                        absolute_delta_percentile: difference.absolute_delta_percentile,
                        formula: DIFFERENCE_FORMULA_ID,
                        baseline: DIFFERENCE_BASELINE_ID,
                        baseline_total: evidence_v4.cohort.full_row_count as u64,
                        active_total: evidence_v4.cohort.included_row_count as u64,
                    }
                }),
                row_id_sample: pinned.summary.hit.row_ids.iter().copied().collect(),
                sample_limit: ScatterInspectionConfig::default().max_row_ids_per_bin,
                evidence_key_values: pinned
                    .evidence_keys
                    .iter()
                    .map(|key| key.value.clone())
                    .collect(),
            }
        });
        ScatterSelectionEvidenceV5::from_v4(evidence_v4, session_context, pinned_inspection).ok()
    }
}
