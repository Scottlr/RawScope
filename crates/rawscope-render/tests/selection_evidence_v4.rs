use rawscope_core::{F32Range, RowId};
use rawscope_data::{DatasetFilter, DatasetIdentity, ScatterProjection};
use rawscope_evidence::{
    ScatterEvidenceView, ScatterSelectionEvidenceV2, ScatterSelectionKindCounts,
};
use rawscope_render::{
    scatter_selection_evidence_v3_json, scatter_selection_evidence_v3_markdown,
    scatter_selection_evidence_v4_json, scatter_selection_evidence_v4_markdown, ComparisonRatio,
    DensityEncoding, DifferenceDensityEvidenceConfig, PinnedScatterInspectionEvidence,
    PointRevealEvidence, PointRevealMode, ReliefFieldConfig, ScatterAggregateEvidenceContext,
    ScatterCohortEvidence, ScatterDensityMode, ScatterDensityPresentation, ScatterKindComparison,
    ScatterSelectionComparison, ScatterSelectionEvidenceV3, ScatterSelectionEvidenceV4,
    ScatterSelectionEvidenceV4Error, ScatterVisualQueryV4, DIFFERENCE_BASELINE_ID,
    DIFFERENCE_FORMULA_ID,
};

#[test]
fn legacy_scatter_evidence_outputs_remain_unchanged() {
    let v3 = sample_v3();
    let json = scatter_selection_evidence_v3_json(&v3).unwrap();
    let markdown = scatter_selection_evidence_v3_markdown(&v3);
    assert!(json.contains("\"schema_version\": 3"));
    assert!(!json.contains("visual_query"));
    assert!(markdown.starts_with("# RawScope Scatter Selection Evidence\n"));
    assert!(!markdown.contains("Evidence v4"));
}

#[test]
fn v4_json_serializes_filtered_visual_query() {
    let evidence = sample_v4(ScatterDensityMode::FilteredDifference);
    let json = scatter_selection_evidence_v4_json(&evidence).unwrap();
    let beta = json.find("beta").unwrap();
    let zeta = json.find("zeta").unwrap();
    assert!(json.contains("rawscope.scatter-selection-evidence.v4"));
    assert!(json.contains("active_share_minus_full_baseline_share"));
    assert!(json.contains("mean_difference"));
    assert!(
        beta < zeta,
        "category values must serialize in sorted order"
    );
    assert!(!json.contains("filter_revision"));
}

#[test]
fn v4_markdown_discloses_difference_formula_and_sampling() {
    let markdown =
        scatter_selection_evidence_v4_markdown(&sample_v4(ScatterDensityMode::FilteredDifference))
            .unwrap();
    assert!(markdown.contains("Formula: `active_share_minus_full_baseline_share`"));
    assert!(markdown.contains("Sampled: true"));
    assert!(markdown.contains("Rendered points: 0"));
}

#[test]
fn v4_omits_relief_when_presentation_is_not_relief() {
    let evidence = sample_v4(ScatterDensityMode::AbsoluteDensity);
    let json = scatter_selection_evidence_v4_json(&evidence).unwrap();
    assert!(!json.contains("\"relief\""));
    assert!(!scatter_selection_evidence_v4_markdown(&evidence)
        .unwrap()
        .contains("Relief Shading"));
}

#[test]
fn v4_serializes_relief_and_pinned_inspection_when_active() {
    let mut query = difference_query();
    query.density_mode = ScatterDensityMode::AbsoluteDensity;
    query.density_presentation = ScatterDensityPresentation::ReliefField;
    query.difference = None;
    query.point_reveal.rendered_count = 4;
    query.relief = Some(ReliefFieldConfig::default());
    let evidence = ScatterSelectionEvidenceV4::from_v3(
        &sample_v3(),
        query,
        ScatterCohortEvidence {
            full_row_count: 20,
            included_row_count: 8,
            excluded_row_count: 12,
        },
        Some(PinnedScatterInspectionEvidence {
            bin_x: 3,
            bin_y: 7,
            x_range: F32Range::new(1.0, 2.0),
            y_range: F32Range::new(2.0, 3.0),
            row_count: 9,
            row_id_sample: vec![RowId(3), RowId(5)],
            sample_limit: 16,
        }),
    )
    .unwrap();
    let json = scatter_selection_evidence_v4_json(&evidence).unwrap();
    assert!(json.contains("\"relief\""));
    assert!(json.contains("\"pinned_inspection\""));
    assert!(json.contains("\"sample_limit\": 16"));
}

#[test]
fn v4_rejects_incoherent_difference_without_filter() {
    let v3 = sample_v3();
    let mut query = difference_query();
    query.filters.clear();
    assert_eq!(
        ScatterSelectionEvidenceV4::from_v3(
            &v3,
            query,
            ScatterCohortEvidence {
                full_row_count: 20,
                included_row_count: 8,
                excluded_row_count: 12,
            },
            None,
        ),
        Err(ScatterSelectionEvidenceV4Error::DifferenceRequiresFilter)
    );
}

#[test]
fn v4_validation_rejects_mutated_schema_and_percentage() {
    let mut evidence = sample_v4(ScatterDensityMode::AbsoluteDensity);
    evidence.schema_version = 99;
    assert_eq!(
        evidence.validate(),
        Err(ScatterSelectionEvidenceV4Error::InvalidSchemaVersion)
    );

    let mut evidence = sample_v4(ScatterDensityMode::AbsoluteDensity);
    evidence.selected_percentage = f32::NAN;
    assert_eq!(
        evidence.validate(),
        Err(ScatterSelectionEvidenceV4Error::InvalidSelectedPercentage)
    );
}

#[test]
fn v4_validation_rejects_samples_larger_than_selected_count() {
    let mut evidence = sample_v4(ScatterDensityMode::AbsoluteDensity);
    evidence.selected_row_id_sample = vec![RowId(1), RowId(2)];
    evidence.selected_row_count = 1;
    assert_eq!(
        evidence.validate(),
        Err(ScatterSelectionEvidenceV4Error::InvalidSelectedSample)
    );
}

fn sample_v4(mode: ScatterDensityMode) -> ScatterSelectionEvidenceV4 {
    let query = match mode {
        ScatterDensityMode::AbsoluteDensity => ScatterVisualQueryV4 {
            density_mode: mode,
            density_presentation: ScatterDensityPresentation::TopographicField,
            difference: None,
            point_reveal: PointRevealEvidence {
                mode: PointRevealMode::Auto,
                eligible_count: 20,
                rendered_count: 12,
                sampled: true,
            },
            relief: None,
            ..difference_query()
        },
        ScatterDensityMode::FilteredDifference => difference_query(),
    };
    ScatterSelectionEvidenceV4::from_v3(
        &sample_v3(),
        query,
        ScatterCohortEvidence {
            full_row_count: 20,
            included_row_count: 8,
            excluded_row_count: 12,
        },
        None,
    )
    .unwrap()
}

fn difference_query() -> ScatterVisualQueryV4 {
    ScatterVisualQueryV4 {
        x_range: F32Range::new(0.0, 10.0),
        y_range: F32Range::new(-5.0, 5.0),
        grid_width: 16,
        grid_height: 16,
        projection: ScatterProjection::MeanDifference,
        filters: vec![DatasetFilter::Categories {
            column_name: "category".into(),
            included_values: vec!["zeta".into(), "beta".into()],
            include_missing: false,
        }],
        density_mode: ScatterDensityMode::FilteredDifference,
        density_encoding: DensityEncoding::scatter_default(),
        density_presentation: ScatterDensityPresentation::TopographicField,
        difference: Some(DifferenceDensityEvidenceConfig {
            formula: DIFFERENCE_FORMULA_ID,
            baseline: DIFFERENCE_BASELINE_ID,
            baseline_total: 20,
            active_total: 8,
            max_abs_delta: 0.125,
        }),
        point_reveal: PointRevealEvidence {
            mode: PointRevealMode::Auto,
            eligible_count: 8,
            rendered_count: 0,
            sampled: true,
        },
        relief: None,
    }
}

fn sample_v3() -> ScatterSelectionEvidenceV3 {
    let v2 = ScatterSelectionEvidenceV2 {
        dataset_identity: DatasetIdentity::synthetic_scatter(42, 20),
        view: ScatterEvidenceView {
            x_range: F32Range::new(0.0, 10.0),
            y_range: F32Range::new(-5.0, 5.0),
            grid_width: 16,
            grid_height: 16,
        },
        selected_row_count: 1,
        selected_percentage: 5.0,
        selected_row_id_sample: vec![RowId(3)],
        selected_record_sample: vec![],
        selected_source_column_names: vec![],
        selected_source_row_sample: vec![],
        point_kind_counts: ScatterSelectionKindCounts::default(),
        top_point_kind: None,
        selected_x_range: None,
        selected_y_range: None,
        brush_x_range: F32Range::new(1.0, 2.0),
        brush_y_range: F32Range::new(1.0, 2.0),
    };
    let zero = ComparisonRatio {
        selected_count: 0,
        baseline_count: 0,
        selected_percentage: 0.0,
        baseline_percentage: 0.0,
        delta_percentage_points: 0.0,
    };
    ScatterSelectionEvidenceV3::from_v2_with_presentation(
        &v2,
        DensityEncoding::scatter_default(),
        ScatterDensityPresentation::TopographicField,
        ScatterSelectionComparison {
            selected_row_count: 1,
            baseline_row_count: 20,
            selected_percentage: 5.0,
            point_kind_ratios: ScatterKindComparison {
                cluster: zero,
                background: zero,
                outlier: zero,
                unclassified: zero,
            },
        },
        ScatterAggregateEvidenceContext {
            bin_limit: 16,
            bins: vec![],
        },
        None,
    )
}
