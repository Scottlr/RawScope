use proptest::prelude::*;
use rawscope_analysis::{cohort::CohortGenerationCounter, selection::SelectionSnapshot};
use rawscope_core::{RowId, SelectionId};
use rawscope_data::DatasetGenerationCounter;

proptest! {
    #[test]
    fn selection_membership_is_sorted_unique_and_sampled_from_membership(
        row_ids in proptest::collection::vec(0_u64..10_000, 0..64),
        bins in proptest::collection::vec(0_u32..256, 0..64),
        sample_limit in 1_usize..32,
    ) {
        let snapshot = SelectionSnapshot::from_parts(
            DatasetGenerationCounter::default().mint(),
            CohortGenerationCounter::default().mint(),
            SelectionId(1),
            row_ids.iter().copied().map(RowId),
            bins,
            sample_limit,
        ).expect("positive sample limit is valid");

        prop_assert!(snapshot.row_ids().windows(2).all(|pair| pair[0] < pair[1]));
        prop_assert!(snapshot.selected_bins().windows(2).all(|pair| pair[0] < pair[1]));
        prop_assert!(snapshot.samples().iter().all(|row| snapshot.row_ids().binary_search(row).is_ok()));
        prop_assert!(snapshot.samples().len() <= sample_limit);
    }
}
