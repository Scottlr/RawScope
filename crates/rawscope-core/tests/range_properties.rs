use proptest::prelude::*;
use rawscope_core::{F32Domain, U64Range};

proptest! {
    #[test]
    fn ordered_u64_bounds_produce_a_domain(min in 0_u64..1_000_000, span in 1_u64..1_000_000) {
        let max = min + span;
        let domain = U64Range::try_new(min, max).expect("bounded positive span is valid");
        prop_assert_eq!(domain.min(), min);
        prop_assert_eq!(domain.max(), max);
        prop_assert!(domain.contains(min));
        prop_assert!(domain.contains(max));
    }

    #[test]
    fn ordered_f32_bounds_have_positive_span(min in -1_000_000_f32..1_000_000_f32, span in 0.001_f32..1_000_000_f32) {
        let max = min + span;
        let domain = F32Domain::try_new(min, max).expect("bounded finite span is valid");
        prop_assert!(domain.max() > domain.min());
        prop_assert!(domain.contains(min));
        prop_assert!(domain.contains(max));
    }
}
