use rawscope_data::ScatterProjection;
use rawscope_render::{
    ComparisonFieldRenderStats, DensityEncoding, DensityPalette, DensityTransform, PointRevealMode,
    PointRevealStats, ReliefFieldConfig, ScatterDensityMode, ScatterDensityPresentation,
};

use super::DensityEncodingUiState;

#[test]
fn scatter_encoding_makes_density_transform_and_range_visible() {
    let encoding = DensityEncodingUiState::scatter(
        DensityEncoding::scatter_default(),
        ScatterDensityPresentation::TopographicField,
        256,
        256,
        42,
        true,
        PointRevealMode::Auto,
        None,
        Some(ScatterProjection::RawXY),
        ScatterDensityMode::AbsoluteDensity,
        false,
        None,
        ReliefFieldConfig::default(),
    );

    assert_eq!(encoding.surface_label, "Scatter density");
    assert_eq!(encoding.bin_grid_label, "Grid 256x256 bins");
    assert_eq!(
        encoding.scale_label,
        "Scale log1p(count), normalized to viewport max"
    );
    assert_eq!(encoding.palette_label, "Palette scatter sequential");
    assert_eq!(encoding.range_label, "Range 0..42 rows/bin");
    assert_eq!(encoding.max_label, "max 42");
    assert_eq!(
        encoding.scatter_presentation,
        Some(ScatterDensityPresentation::TopographicField)
    );
    assert_eq!(encoding.encoding.palette, DensityPalette::ScatterSequential);
}

#[test]
fn timeline_encoding_uses_event_units() {
    let encoding =
        DensityEncodingUiState::timeline(DensityEncoding::timeline_default(), 256, 12, 7);

    assert_eq!(encoding.surface_label, "Timeline density");
    assert_eq!(encoding.measure_label, "Events per bin");
    assert_eq!(encoding.bin_grid_label, "Grid 256x12 bins");
    assert_eq!(encoding.range_label, "Range 0..7 events/bin");
    assert_eq!(
        encoding.encoding.palette,
        DensityPalette::TimelineSequential
    );
}

#[test]
fn encoding_state_reflects_active_transform() {
    let active_encoding =
        DensityEncoding::scatter_default().with_transform(DensityTransform::Linear);
    let encoding = DensityEncodingUiState::scatter(
        active_encoding,
        ScatterDensityPresentation::ExactCells,
        128,
        128,
        8,
        true,
        PointRevealMode::Off,
        None,
        None,
        ScatterDensityMode::AbsoluteDensity,
        false,
        None,
        ReliefFieldConfig::default(),
    );

    assert_eq!(encoding.encoding.transform, DensityTransform::Linear);
    assert_eq!(encoding.point_reveal_mode, Some(PointRevealMode::Off));
    assert_eq!(
        encoding.scale_label,
        "Scale linear(count), normalized to viewport max"
    );
}

#[test]
fn point_reveal_stats_are_projected_without_threshold_controls() {
    let stats = PointRevealStats {
        eligible_count: 94_499,
        rendered_count: 50_000,
        sampled: true,
        blend: 0.27,
    };
    let encoding = DensityEncodingUiState::scatter(
        DensityEncoding::scatter_default(),
        ScatterDensityPresentation::TopographicField,
        256,
        256,
        100,
        true,
        PointRevealMode::Auto,
        Some(stats),
        Some(ScatterProjection::RawXY),
        ScatterDensityMode::AbsoluteDensity,
        true,
        None,
        ReliefFieldConfig::default(),
    );

    assert_eq!(encoding.point_reveal_mode, Some(PointRevealMode::Auto));
    assert_eq!(encoding.point_reveal_stats, Some(stats));
}

#[test]
fn lichess_projection_options_include_mean_difference() {
    let encoding = DensityEncodingUiState::scatter(
        DensityEncoding::scatter_default(),
        ScatterDensityPresentation::TopographicField,
        256,
        256,
        10,
        true,
        PointRevealMode::Auto,
        None,
        Some(ScatterProjection::MeanDifference),
        ScatterDensityMode::AbsoluteDensity,
        true,
        None,
        ReliefFieldConfig::default(),
    );

    assert_eq!(
        encoding.scatter_projection,
        Some(ScatterProjection::MeanDifference)
    );
}

#[test]
fn difference_mode_projects_cohort_totals_and_availability() {
    let stats = ComparisonFieldRenderStats {
        baseline_total: 200_000,
        active_total: 94_499,
        baseline_recompute_count: 1,
    };
    let encoding = DensityEncodingUiState::scatter(
        DensityEncoding::scatter_default(),
        ScatterDensityPresentation::TopographicField,
        256,
        256,
        10,
        true,
        PointRevealMode::Auto,
        None,
        Some(ScatterProjection::RawXY),
        ScatterDensityMode::FilteredDifference,
        true,
        Some(stats),
        ReliefFieldConfig::default(),
    );

    assert_eq!(
        encoding.scatter_density_mode,
        Some(ScatterDensityMode::FilteredDifference)
    );
    assert!(encoding.difference_available);
    assert_eq!(encoding.difference_stats, Some(stats));
}
