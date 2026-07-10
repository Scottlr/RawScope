use super::{destination_count_buffer, DensityReadbackPolicy};

#[test]
fn no_readback_update_does_not_request_count_mapping() {
    assert!(!DensityReadbackPolicy::None.requests_count_mapping());
    assert!(!DensityReadbackPolicy::MaxOnly.requests_count_mapping());
    assert!(DensityReadbackPolicy::FullCounts.requests_count_mapping());
}

#[test]
fn count_buffer_swap_preserves_completed_source_field() {
    let active = 0usize;
    let destination = destination_count_buffer(active);
    assert_ne!(active, destination);
    assert_eq!(destination_count_buffer(destination), active);
}
