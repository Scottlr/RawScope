//! Neutral, deterministic generic evidence fixture for benchmark scenarios.

use rawscope_analysis::{cohort::CohortGenerationCounter, selection::SelectionSnapshot};
use rawscope_core::{RowId, SelectionId};
use rawscope_data::{DatasetGenerationCounter, DatasetIdentity};
use rawscope_evidence::{
    CategoryCompositionEvidenceV1, CategoryLayerEvidenceV1, CohortComparisonEvidenceV1,
    ComparisonSplitEvidenceV1, DensityModeEvidenceV1, DensityRidgeEvidenceV1, EvidenceContext,
    EvidenceDocument, EvidenceVisualContext, ExactFieldEvidenceV1, FieldDomainEvidenceV1,
    FieldEvidenceV1, FieldKindEvidenceV1, FreshnessEvidenceV1, MassContourEvidenceV1,
    NumericDomainEvidenceV1, QualityTierEvidenceV1, RowCountAggregationEvidenceV1,
    SemanticPointSampleEvidenceV1, VisualFieldEvidenceV1, VisualFieldModeEvidenceV1,
    VisualFieldProjectionEvidenceV1, VisualFieldProvenanceEvidenceV1,
};

pub fn evidence() -> VisualFieldEvidenceV1 {
    let mut dataset_generations = DatasetGenerationCounter::default();
    let dataset_generation = dataset_generations.mint();
    let mut cohort_generations = CohortGenerationCounter::default();
    let cohort_generation = cohort_generations.mint();
    let snapshot = SelectionSnapshot::from_parts(
        dataset_generation,
        cohort_generation,
        SelectionId(42),
        (0..256).map(RowId),
        [1, 3, 5, 8],
        16,
    )
    .expect("benchmark selection fixture is valid");
    let document = EvidenceDocument::from_selection(
        &snapshot,
        EvidenceContext {
            dataset_generation,
            cohort_generation,
            dataset_identity: DatasetIdentity::synthetic_scatter(42, 1_024),
            cohort_included_row_count: 1_024,
            source_rows_available: true,
            visual: EvidenceVisualContext {
                x_min: -10.0,
                x_max: 10.0,
                y_min: -10.0,
                y_max: 10.0,
                grid_width: 128,
                grid_height: 96,
            },
        },
    )
    .expect("benchmark evidence document is valid");
    let field = |column_id: u32, display_name: &str| FieldEvidenceV1 {
        column_id,
        display_name: display_name.to_string(),
        kind: FieldKindEvidenceV1::Numeric,
        domain: FieldDomainEvidenceV1::Numeric {
            min: -10.0,
            max: 10.0,
        },
        missing_count: 8,
        invalid_count: 2,
        projected_count: 1_014,
        time_quantization: None,
    };
    VisualFieldEvidenceV1::from_document(
        &document,
        VisualFieldProjectionEvidenceV1::NumericPair {
            x: field(0, "horizontal"),
            y: field(1, "vertical"),
        },
        None,
        RowCountAggregationEvidenceV1 {
            eligible_row_count: 1_024,
            counted_row_count: 1_014,
            missing_row_count: 8,
            invalid_row_count: 2,
            weighted: false,
        },
        ExactFieldEvidenceV1 {
            field_generation: 7,
            mapping_identity: "numeric-pair:0:1".to_string(),
            dataset_generation: dataset_generation.get(),
            cohort_generation: cohort_generation.get(),
            width: 128,
            height: 96,
            x_domain: NumericDomainEvidenceV1 {
                min: -10.0,
                max: 10.0,
            },
            y_domain: NumericDomainEvidenceV1 {
                min: -10.0,
                max: 10.0,
            },
            bin_rule: "floor(normalized * dimension), max-inclusive edge clamp".to_string(),
            quality: QualityTierEvidenceV1::Exact,
            freshness: FreshnessEvidenceV1::Settled,
        },
        VisualFieldModeEvidenceV1::Density(DensityModeEvidenceV1 {
            transform: "log1p(count)".to_string(),
            normalization: "viewport_max".to_string(),
            palette: "neutral-sequential-v1".to_string(),
            contours: vec![MassContourEvidenceV1 {
                requested_fraction_basis_points: 9_500,
                actual_enclosed_row_count: 972,
                total_row_count: 1_014,
                minimum_bin_count: Some(2),
            }],
            marginal_x_totals: vec![8; 128],
            marginal_y_totals: vec![10; 96],
            semantic_point_sample: SemanticPointSampleEvidenceV1 {
                eligible_count: 1_014,
                rendered_count: 256,
                sample_complete: false,
            },
        }),
        None,
        VisualFieldProvenanceEvidenceV1 {
            mapping_identity: "numeric-pair:0:1".to_string(),
            field_generation: 7,
            source: "neutral-benchmark-fixture".to_string(),
            construction: "settled-count-grid".to_string(),
        },
    )
    .expect("benchmark visual-field evidence is valid")
}

/// Returns one canonical evidence artifact for each closed generic mode and
/// projection shape. Every variant shares the same immutable document identity
/// so the export benchmark exercises the real wire/report validation path.
pub fn evidence_variants() -> Vec<(&'static str, VisualFieldEvidenceV1)> {
    let base = evidence();

    let mut composition = base.clone();
    composition.category = Some(category_field());
    composition.presentation =
        VisualFieldModeEvidenceV1::CategoryComposition(CategoryCompositionEvidenceV1 {
            layers: vec![
                CategoryLayerEvidenceV1 {
                    index: 0,
                    value: Some("alpha".to_string()),
                    display_label: "alpha".to_string(),
                    row_count: 412,
                    reserved: None,
                },
                CategoryLayerEvidenceV1 {
                    index: 1,
                    value: None,
                    display_label: "Other".to_string(),
                    row_count: 398,
                    reserved: Some(rawscope_evidence::CategoryReservedLayerV1::Other),
                },
                CategoryLayerEvidenceV1 {
                    index: 2,
                    value: None,
                    display_label: "Missing".to_string(),
                    row_count: 96,
                    reserved: Some(rawscope_evidence::CategoryReservedLayerV1::Missing),
                },
                CategoryLayerEvidenceV1 {
                    index: 3,
                    value: None,
                    display_label: "Invalid".to_string(),
                    row_count: 108,
                    reserved: Some(rawscope_evidence::CategoryReservedLayerV1::Invalid),
                },
            ],
            index_accuracy: "exact-index-v1".to_string(),
            dominant_real_layer_rule: "max count, lowest index on tie".to_string(),
            fixed_visible_layer_count: 4,
            purity_formula: "1 - H(p) / ln(fixed_visible_layer_count)".to_string(),
            total_density_lightness: "density lightness from total count".to_string(),
        });
    composition
        .validate()
        .expect("composition benchmark evidence should be valid");

    let mut comparison = base.clone();
    comparison.presentation =
        VisualFieldModeEvidenceV1::CohortComparison(CohortComparisonEvidenceV1 {
            baseline_identity: "baseline:fixture".to_string(),
            active_identity: "active:fixture".to_string(),
            baseline_row_count: 1_100,
            active_row_count: 1_014,
            signed_delta_formula: "active_count/active_total - baseline_count/baseline_total"
                .to_string(),
            support_share_formula: "(active_share + baseline_share) / 2".to_string(),
            shared_density_maximum: 12.0,
            shared_delta_maximum: 1.0,
            shared_support_maximum: 1.0,
            split: Some(ComparisonSplitEvidenceV1 {
                fraction: 0.5,
                signed_side: true,
            }),
        });
    comparison
        .validate()
        .expect("comparison benchmark evidence should be valid");

    let mut ridges = base.clone();
    ridges.presentation = VisualFieldModeEvidenceV1::DensityRidges(DensityRidgeEvidenceV1 {
        scale: "medium".to_string(),
        formula_version: 1,
        minimum_strength_basis_points: 250,
        minimum_anisotropy_basis_points: 1_500,
        local_maximum_rule: "discrete 8-neighbour scalar-field maximum".to_string(),
        orientation: "undirected normalized-field tangent; not a direction or trajectory"
            .to_string(),
    });
    ridges
        .validate()
        .expect("ridge benchmark evidence should be valid");

    let mut time_value = base;
    let time = field(3, "observed_at");
    time_value.projection = VisualFieldProjectionEvidenceV1::TimeValue {
        time,
        value: field(1, "measurement"),
    };
    time_value.field.mapping_identity = "time-value:3:1".to_string();
    time_value.provenance.mapping_identity = "time-value:3:1".to_string();
    time_value
        .validate()
        .expect("time-value benchmark evidence should be valid");

    vec![
        ("density", evidence()),
        ("composition", composition),
        ("comparison", comparison),
        ("ridges", ridges),
        ("time-value", time_value),
    ]
}

fn category_field() -> FieldEvidenceV1 {
    FieldEvidenceV1 {
        column_id: 2,
        display_name: "segment".to_string(),
        kind: FieldKindEvidenceV1::Category,
        domain: FieldDomainEvidenceV1::Categorical { distinct_count: 7 },
        missing_count: 96,
        invalid_count: 108,
        projected_count: 810,
        time_quantization: None,
    }
}

fn field(column_id: u32, display_name: &str) -> FieldEvidenceV1 {
    FieldEvidenceV1 {
        column_id,
        display_name: display_name.to_string(),
        kind: if display_name == "observed_at" {
            FieldKindEvidenceV1::TimestampMicros
        } else {
            FieldKindEvidenceV1::Numeric
        },
        domain: if display_name == "observed_at" {
            FieldDomainEvidenceV1::TimestampMicros {
                min: 1_700_000_000_000_000,
                max: 1_700_000_500_000_000,
            }
        } else {
            FieldDomainEvidenceV1::Numeric {
                min: -10.0,
                max: 10.0,
            }
        },
        missing_count: 8,
        invalid_count: 2,
        projected_count: 1_014,
        time_quantization: None,
    }
}
