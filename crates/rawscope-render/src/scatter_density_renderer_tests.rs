use crate::DensityTransform;

#[test]
fn density_intensity_maps_empty_bins_to_zero() {
    assert_eq!(
        crate::density_intensity(0, 12, DensityTransform::Log1p),
        0.0
    );
    assert_eq!(crate::density_intensity(4, 0, DensityTransform::Log1p), 0.0);
}

#[test]
fn log1p_density_intensity_maps_max_count_to_one() {
    assert_eq!(
        crate::density_intensity(12, 12, DensityTransform::Log1p),
        1.0
    );
}

#[test]
fn log1p_density_intensity_keeps_mid_counts_visible() {
    let log_scaled = crate::density_intensity(1, 4, DensityTransform::Log1p);
    assert!(log_scaled > 0.25);
    assert!(log_scaled < 1.0);
}
