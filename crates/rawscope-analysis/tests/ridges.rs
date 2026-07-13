use rawscope_analysis::visual_field::{derive_density_ridges, RidgeConfig, RidgeScale};
use rawscope_core::{DensityCountGrid, GridSize};

fn grid(width: u32, height: u32, counts: &[u32]) -> DensityCountGrid {
    DensityCountGrid::new(GridSize::new(width, height), counts.to_vec())
}

fn permissive(scale: RidgeScale) -> RidgeConfig {
    RidgeConfig {
        scale,
        minimum_strength_basis_points: 0,
        minimum_anisotropy_basis_points: 1,
    }
}

#[test]
fn flat_and_empty_fields_have_no_ridges() {
    for scale in [RidgeScale::Fine, RidgeScale::Medium, RidgeScale::Coarse] {
        let field = derive_density_ridges(&grid(7, 5, &[0; 35]), permissive(scale)).unwrap();
        assert!(field.cells.iter().all(|cell| cell.strength == 0.0));
        assert!(field
            .cells
            .iter()
            .all(|cell| cell.tangent_x == 0.0 && cell.tangent_y == 0.0));
    }
}

#[test]
fn straight_band_has_undirected_tangent_along_the_band() {
    let mut counts = [0; 81];
    for y in 3..6 {
        for x in 1..8 {
            counts[y * 9 + x] = 10;
        }
    }
    let field = derive_density_ridges(&grid(9, 9, &counts), permissive(RidgeScale::Fine)).unwrap();
    let cell = field.cells[4 * 9 + 4];
    assert!(cell.strength > 0.0);
    assert!(cell.tangent_x.abs() > 0.9);
    assert!(cell.tangent_y.abs() < 0.2);
}

#[test]
fn diagonal_band_orientation_is_sign_canonical() {
    let mut counts = [0; 81];
    for y in 1..8 {
        for x in 1..8 {
            if (x as i32 - y as i32).abs() <= 1 {
                counts[y * 9 + x] = 10;
            }
        }
    }
    let field = derive_density_ridges(&grid(9, 9, &counts), permissive(RidgeScale::Fine)).unwrap();
    let cell = field.cells[4 * 9 + 4];
    assert!(cell.strength > 0.0);
    assert!(cell.tangent_x >= 0.0);
    assert!((cell.tangent_x * cell.tangent_x + cell.tangent_y * cell.tangent_y - 1.0).abs() < 1e-5);
}

#[test]
fn ridge_scale_suppresses_features_below_its_support() {
    let mut counts = [0; 225];
    for y in 1..14 {
        for x in 1..14 {
            if (x + y) % 2 == 0 {
                counts[y * 15 + x] = 100;
            }
        }
    }
    let fine = derive_density_ridges(&grid(15, 15, &counts), permissive(RidgeScale::Fine)).unwrap();
    let coarse =
        derive_density_ridges(&grid(15, 15, &counts), permissive(RidgeScale::Coarse)).unwrap();
    let fine_strength = fine.cells.iter().map(|cell| cell.strength).sum::<f32>();
    let fine_nonzero = fine.cells.iter().filter(|cell| cell.strength > 0.0).count();
    let coarse_nonzero = coarse
        .cells
        .iter()
        .filter(|cell| cell.strength > 0.0)
        .count();
    assert!(fine_strength > 0.0);
    assert!(coarse_nonzero < fine_nonzero);
}

#[test]
fn isotropic_peak_and_crossing_center_do_not_invent_orientation() {
    let mut impulse = [0; 49];
    impulse[3 * 7 + 3] = 100;
    let field = derive_density_ridges(&grid(7, 7, &impulse), permissive(RidgeScale::Fine)).unwrap();
    let center = field.cells[3 * 7 + 3];
    assert_eq!((center.tangent_x, center.tangent_y), (0.0, 0.0));

    let mut crossing = [0; 49];
    for index in 0..7 {
        crossing[3 * 7 + index] = 10;
        crossing[index * 7 + 3] = 10;
    }
    let crossing_field =
        derive_density_ridges(&grid(7, 7, &crossing), permissive(RidgeScale::Fine)).unwrap();
    let crossing_center = crossing_field.cells[3 * 7 + 3];
    assert_eq!(
        (crossing_center.tangent_x, crossing_center.tangent_y),
        (0.0, 0.0)
    );
}

#[test]
fn height_ridge_requires_local_maximum_across_minor_eigenvector() {
    let mut counts = [0; 49];
    for y in 0..7 {
        for x in 0..7 {
            counts[y * 7 + x] = x as u32;
        }
    }
    let field = derive_density_ridges(&grid(7, 7, &counts), permissive(RidgeScale::Fine)).unwrap();
    assert_eq!(field.cells[3 * 7 + 3].strength, 0.0);
}

#[test]
fn edge_cells_follow_documented_boundary_rule() {
    let counts = grid(3, 3, &[1, 2, 3, 1, 2, 3, 1, 2, 3]);
    let field = derive_density_ridges(&counts, permissive(RidgeScale::Fine)).unwrap();
    assert_eq!(field.cells.len(), 9);
    assert!(field.cells.iter().all(|cell| cell.strength.is_finite()));
}

#[test]
fn ridge_values_are_finite_bounded_and_deterministic() {
    let counts = grid(
        8,
        6,
        &[
            0, 1, 3, 2, 8, 1, 0, 4, 1, 7, 2, 3, 1, 1, 9, 0, 2, 1, 4, 3, 0, 1, 2, 8, 1, 1, 5, 2, 3,
            0, 1, 2, 4, 7, 1, 0, 3, 2, 2, 1, 0, 1, 4, 3, 2, 1, 5, 0,
        ],
    );
    let config = permissive(RidgeScale::Medium);
    let first = derive_density_ridges(&counts, config).unwrap();
    let second = derive_density_ridges(&counts, config).unwrap();
    assert_eq!(first, second);
    for cell in first.cells.iter() {
        assert!(cell.strength.is_finite() && (0.0..=1.0).contains(&cell.strength));
        assert!(cell.tangent_x.is_finite() && cell.tangent_y.is_finite());
        let length = (cell.tangent_x * cell.tangent_x + cell.tangent_y * cell.tangent_y).sqrt();
        assert!(cell.strength == 0.0 || (length - 1.0).abs() < 1e-5);
    }
}

#[test]
fn strength_cutoff_removes_weak_candidates_after_field_normalization() {
    let counts = grid(
        8,
        6,
        &[
            0, 1, 3, 2, 8, 1, 0, 4, 1, 7, 2, 3, 1, 1, 9, 0, 2, 1, 4, 3, 0, 1, 2, 8, 1, 1, 5, 2, 3,
            0, 1, 2, 4, 7, 1, 0, 3, 2, 2, 1, 0, 1, 4, 3, 2, 1, 5, 0,
        ],
    );
    let permissive = RidgeConfig {
        scale: RidgeScale::Medium,
        minimum_strength_basis_points: 0,
        minimum_anisotropy_basis_points: 1,
    };
    let filtered = RidgeConfig {
        minimum_strength_basis_points: 9_000,
        ..permissive
    };
    let permissive_field = derive_density_ridges(&counts, permissive).unwrap();
    let filtered_field = derive_density_ridges(&counts, filtered).unwrap();
    let permissive_count = permissive_field
        .cells
        .iter()
        .filter(|cell| cell.strength > 0.0)
        .count();
    let filtered_count = filtered_field
        .cells
        .iter()
        .filter(|cell| cell.strength > 0.0)
        .count();
    assert!(permissive_count > filtered_count);
    assert!(filtered_count > 0);
}

#[test]
fn ridge_config_rejects_basis_points_above_denominator() {
    assert!(RidgeConfig::try_new(RidgeScale::Fine, 10_001, 0).is_err());
    assert!(RidgeConfig::try_new(RidgeScale::Fine, 0, 10_001).is_err());
}
