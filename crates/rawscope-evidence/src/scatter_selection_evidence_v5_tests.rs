use rawscope_core::{F32Range, RowId};
use rawscope_data::{DatasetFilter, DatasetIdentity, ScatterProjection};

use super::*;
use crate::{
    scatter_selection_evidence_v5_json, scatter_selection_evidence_v5_markdown, ComparisonRatio,
    DensityEncoding, DifferenceDensityEvidenceConfig, PointRevealEvidence, PointRevealMode,
    ScatterCohortEvidence, ScatterDensityPresentation, ScatterSelectionEvidenceV3,
    ScatterSelectionEvidenceV4, ScatterVisualQueryV4,
};

fn absolute_v4() -> ScatterSelectionEvidenceV4 {
    let v3 = ScatterSelectionEvidenceV3 {
        schema_version: 3,
        dataset_identity: DatasetIdentity::synthetic_scatter(1, 100),
        active_dataset_profile: None,
        view: crate::ScatterEvidenceViewV3 {
            x_range: F32Range::new(0.0, 10.0),
            y_range: F32Range::new(0.0, 10.0),
            grid_width: 4,
            grid_height: 4,
            density_encoding: DensityEncoding::scatter_default(),
            density_presentation: ScatterDensityPresentation::ExactCells,
        },
        selected_row_count: 2,
        selected_percentage: 2.0,
        selected_row_id_sample: vec![RowId(1)],
        selected_record_sample: vec![],
        selected_source_column_names: vec![],
        selected_source_row_sample: vec![],
        comparison: crate::ScatterSelectionComparison {
            selected_row_count: 2,
            baseline_row_count: 100,
            selected_percentage: 2.0,
            point_kind_ratios: crate::ScatterKindComparison {
                cluster: zero_ratio(),
                background: zero_ratio(),
                outlier: zero_ratio(),
                unclassified: zero_ratio(),
            },
        },
        aggregate_context: crate::ScatterAggregateEvidenceContext {
            bin_limit: 0,
            bins: vec![],
        },
    };
    ScatterSelectionEvidenceV4::from_v3(
        &v3,
        ScatterVisualQueryV4 {
            x_range: F32Range::new(0.0, 10.0),
            y_range: F32Range::new(0.0, 10.0),
            grid_width: 4,
            grid_height: 4,
            projection: ScatterProjection::RawXY,
            filters: vec![],
            density_mode: ScatterDensityMode::AbsoluteDensity,
            density_encoding: DensityEncoding::scatter_default(),
            density_presentation: ScatterDensityPresentation::ExactCells,
            difference: None,
            point_reveal: PointRevealEvidence {
                mode: PointRevealMode::Off,
                eligible_count: 0,
                rendered_count: 0,
                sampled: false,
            },
            relief: None,
        },
        ScatterCohortEvidence {
            full_row_count: 100,
            included_row_count: 100,
            excluded_row_count: 0,
        },
        None,
    )
    .unwrap()
}

#[test]
fn v5_rejects_incoherent_neighborhood_and_samples() {
    let mut pin = PinnedScatterInspectionEvidenceV5 {
        bin_x: 1,
        bin_y: 1,
        x_range: F32Range::new(0.0, 1.0),
        y_range: F32Range::new(0.0, 1.0),
        row_count: 10,
        active_share: 0.1,
        occupied_density_percentile: Some(0.5),
        neighborhood_radius_bins: 1,
        neighborhood_row_count: 9,
        neighborhood_share: 0.09,
        difference: None,
        row_id_sample: vec![RowId(1)],
        sample_limit: 1,
        evidence_key_values: vec![],
    };
    assert_eq!(
        ScatterSelectionEvidenceV5::from_v4(&absolute_v4(), None, Some(pin.clone())),
        Err(ScatterSelectionEvidenceV5Error::InvalidNeighborhood)
    );
    pin.neighborhood_row_count = 10;
    pin.neighborhood_share = 0.1;
    pin.row_id_sample = vec![RowId(1), RowId(2)];
    assert_eq!(
        ScatterSelectionEvidenceV5::from_v4(&absolute_v4(), None, Some(pin)),
        Err(ScatterSelectionEvidenceV5Error::InvalidSample)
    );
}

#[test]
fn v5_omits_session_context_for_direct_startup() {
    let evidence = ScatterSelectionEvidenceV5::from_v4(&absolute_v4(), None, None).unwrap();
    assert!(evidence.session_context.is_none());
    assert_eq!(evidence.schema_version, 5);
}

#[test]
fn scatter_selection_export_v5_serializes_session_and_rich_pinned_inspection() {
    let pin = PinnedScatterInspectionEvidenceV5 {
        bin_x: 1,
        bin_y: 1,
        x_range: F32Range::new(0.0, 1.0),
        y_range: F32Range::new(0.0, 1.0),
        row_count: 10,
        active_share: 0.1,
        occupied_density_percentile: Some(0.5),
        neighborhood_radius_bins: 1,
        neighborhood_row_count: 20,
        neighborhood_share: 0.2,
        difference: None,
        row_id_sample: vec![RowId(1), RowId(2)],
        sample_limit: 2,
        evidence_key_values: vec!["game-1".to_string(), "game-2".to_string()],
    };
    let context = SessionEvidenceContextV5 {
        session_artifact_kind: "rawscope.session",
        session_schema_version: 1,
        display_name: Some("Lichess".to_string()),
        data_format: SessionDataFormatEvidenceV5::Parquet,
        evidence_key_column: Some("game_id".to_string()),
    };
    let evidence =
        ScatterSelectionEvidenceV5::from_v4(&absolute_v4(), Some(context), Some(pin)).unwrap();
    let json = scatter_selection_evidence_v5_json(&evidence).unwrap();
    assert!(json.contains("rawscope.scatter-selection-evidence.v5"));
    assert!(json.contains("\"session_context\""));
    assert!(json.contains("\"evidence_key_values\""));
    let markdown = scatter_selection_evidence_v5_markdown(&evidence).unwrap();
    assert!(markdown.contains("Occupied-cell density percentile"));
    assert!(markdown.contains("Natural-key values"));
}

#[test]
fn v5_difference_context_matches_active_minus_baseline() {
    let mut source = absolute_v4();
    source.visual_query.filters = vec![DatasetFilter::NumericRange {
        column_name: "rating".to_string(),
        min_inclusive: 1.0,
        max_inclusive: 2.0,
        include_missing: false,
    }];
    source.visual_query.density_mode = ScatterDensityMode::FilteredDifference;
    source.visual_query.difference = Some(DifferenceDensityEvidenceConfig {
        formula: DIFFERENCE_FORMULA_ID,
        baseline: DIFFERENCE_BASELINE_ID,
        baseline_total: 100,
        active_total: 100,
        max_abs_delta: 0.1,
    });
    let pin = PinnedScatterInspectionEvidenceV5 {
        bin_x: 1,
        bin_y: 1,
        x_range: F32Range::new(0.0, 1.0),
        y_range: F32Range::new(0.0, 1.0),
        row_count: 20,
        active_share: 0.2,
        occupied_density_percentile: Some(0.5),
        neighborhood_radius_bins: 1,
        neighborhood_row_count: 30,
        neighborhood_share: 0.3,
        difference: Some(PinnedDifferenceInspectionEvidenceV5 {
            baseline_count: 10,
            active_count: 20,
            baseline_share: 0.1,
            active_share: 0.2,
            share_delta: 0.1,
            direction: DifferenceDirectionEvidenceV5::MoreCommonInActive,
            absolute_delta_percentile: Some(0.75),
            formula: DIFFERENCE_FORMULA_ID,
            baseline: DIFFERENCE_BASELINE_ID,
            baseline_total: 100,
            active_total: 100,
        }),
        row_id_sample: vec![RowId(1)],
        sample_limit: 1,
        evidence_key_values: vec![],
    };
    let evidence = ScatterSelectionEvidenceV5::from_v4(&source, None, Some(pin)).unwrap();
    let json = scatter_selection_evidence_v5_json(&evidence).unwrap();
    assert!(json.contains("more_common_in_active"));
    assert!(json.contains("active_share_minus_full_baseline_share"));
}

fn zero_ratio() -> ComparisonRatio {
    ComparisonRatio {
        selected_count: 0,
        baseline_count: 0,
        selected_percentage: 0.0,
        baseline_percentage: 0.0,
        delta_percentage_points: 0.0,
    }
}
