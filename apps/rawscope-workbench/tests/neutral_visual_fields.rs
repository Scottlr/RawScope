//! Deterministic production-path coverage for the generic visual-field modes.

use std::sync::Arc;

use rawscope_analysis::visual_field::{
    derive_density_ridges, semantic_zoom_frame, summarize_composition_cell,
    summarize_difference_cell, DensityRidgeField, MassFractionBasisPoints, RidgeConfig, RidgeScale,
    SettledDensityContext, TimeAxisTransform, VisualFieldMapping, VisualFieldMode,
    VisualFieldProjection,
};
use rawscope_core::{ColumnId, DensityCountGrid, GridSize};
use rawscope_data::{DatasetSchema, StoreColumnKind};
use rawscope_render::{estimate_visual_field_resources, VisualFieldResourceOptions};

fn neutral_schema_shapes() -> [DatasetSchema; 2] {
    [
        DatasetSchema::try_new([
            ("horizontal", StoreColumnKind::F64),
            ("vertical", StoreColumnKind::F64),
            ("segment", StoreColumnKind::Utf8),
            ("observed_at", StoreColumnKind::TimestampMicros),
        ])
        .expect("numeric neutral schema is valid"),
        DatasetSchema::try_new([
            ("recorded_at", StoreColumnKind::TimestampMicros),
            ("measurement", StoreColumnKind::I64),
            ("group", StoreColumnKind::U64),
        ])
        .expect("time-value neutral schema is valid"),
    ]
}

#[test]
fn neutral_schemas_have_profile_independent_capabilities() {
    let [numeric_schema, time_schema] = neutral_schema_shapes();
    let numeric = VisualFieldMapping::try_new(
        &numeric_schema,
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        },
        Some(ColumnId::new(2)),
    )
    .expect("numeric-pair schema should map without a profile");
    let time_value = VisualFieldMapping::try_new(
        &time_schema,
        VisualFieldProjection::TimeValue {
            time: ColumnId::new(0),
            value: ColumnId::new(1),
        },
        Some(ColumnId::new(2)),
    )
    .expect("time-value schema should map without a profile");
    for mapping in [numeric, time_value] {
        for mode in [
            VisualFieldMode::Density,
            VisualFieldMode::CategoryComposition,
            VisualFieldMode::CohortComparison,
            VisualFieldMode::DensityRidges,
        ] {
            assert_eq!(mapping.require_mode(mode), Ok(()));
        }
    }
}

#[test]
fn all_enabled_modes_publish_real_generation_resources() {
    let grid = Arc::new(DensityCountGrid::new(
        GridSize::new(8, 6),
        vec![
            0, 2, 0, 1, 0, 0, 0, 0, // sparse row
            1, 4, 3, 1, 0, 0, 0, 0, // dense repeated bins
            0, 0, 2, 3, 1, 0, 0, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, 0, 0, 0, 1, 3, 2, 0, 0, 0, 0, 0, 0,
            1, 2, 1,
        ],
    ));
    let settled = SettledDensityContext::try_new(
        17_u64,
        Arc::clone(&grid),
        &[
            MassFractionBasisPoints::try_new(5_000).unwrap(),
            MassFractionBasisPoints::try_new(8_000).unwrap(),
            MassFractionBasisPoints::try_new(9_500).unwrap(),
        ],
    )
    .expect("exact count context should settle once");
    assert_eq!(settled.counts.total_count(), 32);
    assert_eq!(settled.marginals.x_counts.len(), 8);
    assert_eq!(settled.marginals.y_counts.len(), 6);
    let ridge_fields = [RidgeScale::Fine, RidgeScale::Medium, RidgeScale::Coarse]
        .into_iter()
        .map(|scale| {
            derive_density_ridges(
                &settled.counts,
                RidgeConfig {
                    scale,
                    ..RidgeConfig::default()
                },
            )
        })
        .collect::<Result<Vec<DensityRidgeField>, _>>()
        .expect("each reviewed ridge scale should derive");
    assert_eq!(ridge_fields[0].cells.len(), grid.size().bin_count());
    let composition = summarize_composition_cell(&[4, 2, 1, 0]).unwrap();
    assert_eq!(composition.total_count, 7);
    assert_eq!(composition.dominant_layer.unwrap().get(), 0);
    assert_eq!(composition.layers[3].count, 0);
    assert!(composition.purity.is_finite());
    let comparison = summarize_difference_cell(4, 16, 6, 12).unwrap();
    assert!((comparison.signed_delta - (0.5 - 0.25)).abs() < 1.0e-12);
    assert!(comparison.support_share > 0.0);
    let time = TimeAxisTransform::from_domain(1_700_000_000_000_000, 1_700_000_500_000_000)
        .expect("large timestamp domain should remain exact");
    assert_eq!(time.denormalized(1.0), 1_700_000_500_000_000);
    let resources = estimate_visual_field_resources(
        grid.size(),
        VisualFieldResourceOptions {
            category_layers: 4,
            ridge_fields: ridge_fields.len() as u8,
            ..VisualFieldResourceOptions::default()
        },
    )
    .expect("resource accounting should include all derived fields");
    assert!(resources.total_bytes().unwrap() > resources.count_field_bytes);
    assert_eq!(
        semantic_zoom_frame(0.0, Default::default()).point_alpha,
        1.0
    );
}
