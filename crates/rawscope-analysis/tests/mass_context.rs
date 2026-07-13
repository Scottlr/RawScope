use std::sync::Arc;

use rawscope_analysis::visual_field::{
    DensityMarginals, MassContextError, MassContourSet, MassFractionBasisPoints,
    SettledDensityContext,
};
use rawscope_core::{DensityCountGrid, GridSize};

mod mass_context {
    use super::*;

    fn fraction(value: u16) -> MassFractionBasisPoints {
        MassFractionBasisPoints::try_new(value).expect("fixture fraction is valid")
    }

    fn grid(width: u32, height: u32, counts: &[u32]) -> DensityCountGrid {
        DensityCountGrid::new(GridSize::new(width, height), counts.to_vec())
    }

    #[test]
    fn mass_contours_include_complete_threshold_ties() {
        let counts = grid(2, 2, &[5, 5, 2, 1]);
        let requested = [fraction(5_000), fraction(8_000), fraction(9_500)];

        let contours = MassContourSet::from_grid(&counts, &requested).unwrap();

        assert_eq!(contours.levels[0].minimum_bin_count.unwrap().get(), 5);
        assert_eq!(contours.levels[0].enclosed_row_count, 10);
        assert_eq!(contours.levels[0].total_row_count, 13);
        assert_eq!(contours.levels[1].minimum_bin_count.unwrap().get(), 2);
        assert_eq!(contours.levels[1].enclosed_row_count, 12);
        assert_eq!(contours.levels[2].minimum_bin_count.unwrap().get(), 1);
        assert_eq!(contours.levels[2].enclosed_row_count, 13);
    }

    #[test]
    fn mass_contours_reject_invalid_or_unsorted_fractions() {
        assert!(MassFractionBasisPoints::try_new(0).is_err());
        assert!(MassFractionBasisPoints::try_new(10_001).is_err());

        let counts = grid(1, 1, &[1]);
        let duplicate = [fraction(8_000), fraction(8_000)];
        let descending = [fraction(9_500), fraction(5_000)];
        assert!(matches!(
            MassContourSet::from_grid(&counts, &duplicate),
            Err(MassContextError::FractionsNotStrictlyIncreasing { .. })
        ));
        assert!(matches!(
            MassContourSet::from_grid(&counts, &descending),
            Err(MassContextError::FractionsNotStrictlyIncreasing { .. })
        ));
    }

    #[test]
    fn empty_mass_contours_do_not_draw_zero_count_cells() {
        let counts = grid(2, 2, &[0, 0, 0, 0]);
        let contours = MassContourSet::from_grid(&counts, &[fraction(5_000)]).unwrap();

        assert_eq!(contours.levels[0].minimum_bin_count, None);
        assert_eq!(contours.levels[0].enclosed_row_count, 0);
        assert_eq!(contours.levels[0].total_row_count, 0);
    }

    #[test]
    fn marginals_sum_to_total_grid_count() {
        let counts = grid(3, 2, &[1, 2, 3, 4, 5, 6]);
        let marginals = DensityMarginals::from_grid(&counts).unwrap();

        assert_eq!(marginals.x_counts.as_ref(), &[5, 7, 9]);
        assert_eq!(marginals.y_counts.as_ref(), &[6, 15]);
        assert_eq!(marginals.x_counts.iter().sum::<u64>(), counts.total_count());
        assert_eq!(marginals.y_counts.iter().sum::<u64>(), counts.total_count());
        assert_eq!(marginals.max_x_count, 9);
        assert_eq!(marginals.max_y_count, 15);
    }

    #[test]
    fn settled_context_cannot_mix_field_generations() {
        let counts = Arc::new(grid(2, 1, &[3, 1]));
        let first =
            SettledDensityContext::try_new(7_u64, Arc::clone(&counts), &[fraction(5_000)]).unwrap();
        let second =
            SettledDensityContext::try_new(8_u64, Arc::clone(&counts), &[fraction(5_000)]).unwrap();

        assert_eq!(first.field_generation, 7);
        assert_eq!(second.field_generation, 8);
        assert_ne!(first.field_generation, second.field_generation);
        assert!(Arc::ptr_eq(&first.counts, &second.counts));
        assert_eq!(first.contours.levels[0].total_row_count, 4);
        assert_eq!(second.marginals.x_counts.as_ref(), &[3, 1]);
    }
}
