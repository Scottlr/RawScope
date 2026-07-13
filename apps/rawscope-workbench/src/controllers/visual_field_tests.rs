use super::*;
use rawscope_analysis::visual_field::VisualFieldProjection;
use rawscope_core::ColumnId;
use rawscope_data::{DatasetSchema, StoreColumnKind};

fn mapping(time_value: bool, category: bool) -> VisualFieldMapping {
    let schema = if time_value {
        DatasetSchema::try_new([
            ("observed_at", StoreColumnKind::TimestampMicros),
            ("value", StoreColumnKind::F64),
            ("segment", StoreColumnKind::Utf8),
        ])
        .unwrap()
    } else {
        DatasetSchema::try_new([
            ("horizontal", StoreColumnKind::F64),
            ("vertical", StoreColumnKind::F64),
            ("segment", StoreColumnKind::Utf8),
        ])
        .unwrap()
    };
    let projection = if time_value {
        VisualFieldProjection::TimeValue {
            time: ColumnId::new(0),
            value: ColumnId::new(1),
        }
    } else {
        VisualFieldProjection::NumericPair {
            x: ColumnId::new(0),
            y: ColumnId::new(1),
        }
    };
    VisualFieldMapping::try_new(&schema, projection, category.then_some(ColumnId::new(2))).unwrap()
}

#[test]
fn visual_mode_compatibility_matches_closed_matrix() {
    let cases = [
        (VisualFieldMode::Density, true),
        (VisualFieldMode::CategoryComposition, true),
        (VisualFieldMode::CohortComparison, true),
        (VisualFieldMode::DensityRidges, true),
    ];
    for time_value in [false, true] {
        for with_category in [false, true] {
            let map = mapping(time_value, with_category);
            for (mode, expected) in cases {
                let is_available = visual_mode_availability(
                    mode,
                    &map,
                    ComparisonCohortState::Ready,
                    VisualFieldCapabilities::available(),
                    VisualFieldReadiness::Ready,
                ) == VisualModeAvailability::Available;
                assert_eq!(
                    is_available,
                    expected && (with_category || mode != VisualFieldMode::CategoryComposition),
                    "time_value={time_value}, category={with_category}, mode={mode:?}"
                );
            }
        }
    }
}

#[test]
fn profile_identity_never_changes_mode_availability() {
    let numeric = mapping(false, true);
    let time_value = mapping(true, true);
    for mode in [
        VisualFieldMode::Density,
        VisualFieldMode::CategoryComposition,
        VisualFieldMode::CohortComparison,
        VisualFieldMode::DensityRidges,
    ] {
        assert_eq!(
            visual_mode_availability(
                mode,
                &numeric,
                ComparisonCohortState::Ready,
                VisualFieldCapabilities::available(),
                VisualFieldReadiness::Ready,
            ),
            visual_mode_availability(
                mode,
                &time_value,
                ComparisonCohortState::Ready,
                VisualFieldCapabilities::available(),
                VisualFieldReadiness::Ready,
            )
        );
    }
}

#[test]
fn equivalent_commands_do_not_reschedule_generation() {
    let map = mapping(false, true);
    let mut controller = VisualFieldController::new(map);
    assert_eq!(
        controller.handle(VisualFieldCommand::SetMode(VisualFieldMode::Density)),
        VisualFieldCommandResult::Noop
    );
    assert_eq!(
        controller.handle(VisualFieldCommand::SetMapping(map)),
        VisualFieldCommandResult::Noop
    );
    assert_eq!(
        controller.handle(VisualFieldCommand::SetRidgeScale(RidgeScale::Medium)),
        VisualFieldCommandResult::Noop
    );
    assert_eq!(controller.resource_generation(), 0);
}

#[test]
fn split_and_palette_commands_remain_presentation_only() {
    let mut controller = VisualFieldController::new(mapping(false, true));
    let split = ComparisonSplit::new(0.75).unwrap();
    assert_eq!(
        controller.handle(VisualFieldCommand::SetComparisonSplit(split)),
        VisualFieldCommandResult::PresentationOnly
    );
    assert_eq!(
        controller.handle(VisualFieldCommand::SetDensityPresentation(
            ScatterDensityPresentation::TopographicField
        )),
        VisualFieldCommandResult::PresentationOnly
    );
    assert_eq!(controller.resource_generation(), 0);
}

#[test]
fn disabled_mode_reports_specific_reason() {
    let no_category = mapping(false, false);
    assert_eq!(
        visual_mode_availability(
            VisualFieldMode::CategoryComposition,
            &no_category,
            ComparisonCohortState::Ready,
            VisualFieldCapabilities::available(),
            VisualFieldReadiness::Ready,
        ),
        VisualModeAvailability::Unavailable(VisualModeUnavailableReason::CategoryNotMapped)
    );
    let map = mapping(false, true);
    assert_eq!(
        visual_mode_availability(
            VisualFieldMode::CohortComparison,
            &map,
            ComparisonCohortState::Missing,
            VisualFieldCapabilities::available(),
            VisualFieldReadiness::Ready,
        ),
        VisualModeAvailability::Unavailable(VisualModeUnavailableReason::ComparisonCohortMissing)
    );
    assert_eq!(
        visual_mode_availability(
            VisualFieldMode::DensityRidges,
            &map,
            ComparisonCohortState::Ready,
            VisualFieldCapabilities::device_limited(),
            VisualFieldReadiness::Ready,
        ),
        VisualModeAvailability::Unavailable(VisualModeUnavailableReason::DeviceLimit)
    );
}

#[test]
fn time_value_and_numeric_pair_share_controller_path() {
    let numeric = mapping(false, true);
    let time_value = mapping(true, true);
    let mut controller = VisualFieldController::new(numeric);
    assert_eq!(
        controller.handle(VisualFieldCommand::SetMode(
            VisualFieldMode::CategoryComposition
        )),
        VisualFieldCommandResult::ResourceIntentChanged { generation: 1 }
    );
    assert_eq!(
        controller.handle(VisualFieldCommand::SetMapping(time_value)),
        VisualFieldCommandResult::ResourceIntentChanged { generation: 2 }
    );
    assert_eq!(controller.state().mapping(), time_value);
    assert_eq!(
        controller.state().mode(),
        VisualFieldMode::CategoryComposition
    );
}
