use rawscope_core::{F32Range, U64Range};
use rawscope_render::{
    scatter_axes_context, scatter_axes_context_with_options, timeline_axes_context,
    AxisValueFormat, ScatterAxesOptions, ScatterReferenceGuide, ScatterReferenceGuideKind,
};

fn options(guides: Vec<ScatterReferenceGuide>) -> ScatterAxesOptions {
    ScatterAxesOptions {
        target_tick_count: 6,
        x_format: AxisValueFormat::Integer,
        y_format: AxisValueFormat::Integer,
        guides,
    }
}

#[test]
fn nice_ticks_use_one_two_five_steps() {
    let context = scatter_axes_context_with_options(
        F32Range::new(3.0, 97.0),
        F32Range::new(0.0, 1.0),
        "x",
        "y",
        options(Vec::new()),
    );
    assert_eq!(
        context
            .x
            .ticks
            .iter()
            .map(|tick| tick.label.as_str())
            .collect::<Vec<_>>(),
        ["20", "40", "60", "80"]
    );
}

#[test]
fn nice_ticks_handle_negative_overscroll_without_duplicates() {
    let context = scatter_axes_context(
        F32Range::new(-12.0, 108.0),
        F32Range::new(-0.0003, 0.0007),
        "x",
        "y",
        7,
    );
    assert!(context
        .x
        .ticks
        .windows(2)
        .all(|pair| pair[0].fraction < pair[1].fraction));
    assert!(context
        .y
        .ticks
        .windows(2)
        .all(|pair| pair[0].label != pair[1].label));
    assert!(context.x.ticks.len() <= 9);
}

#[test]
fn integer_format_removes_rating_decimal_suffix() {
    let context = scatter_axes_context_with_options(
        F32Range::new(1_500.0, 1_600.0),
        F32Range::new(1_500.0, 1_600.0),
        "x",
        "y",
        options(Vec::new()),
    );
    assert!(context.x.ticks.iter().all(|tick| !tick.label.contains('.')));
}

#[test]
fn compact_format_handles_large_values() {
    let context = scatter_axes_context_with_options(
        F32Range::new(1_000_000.0, 5_000_000.0),
        F32Range::new(0.0, 1.0),
        "x",
        "y",
        ScatterAxesOptions {
            target_tick_count: 5,
            x_format: AxisValueFormat::Compact,
            y_format: AxisValueFormat::Decimal {
                max_fraction_digits: 1,
            },
            guides: Vec::new(),
        },
    );

    assert!(context.x.ticks.iter().all(|tick| tick.label.ends_with('M')));
}

#[test]
fn equality_guide_clips_to_visible_range() {
    let context = scatter_axes_context_with_options(
        F32Range::new(1_000.0, 2_000.0),
        F32Range::new(1_500.0, 2_500.0),
        "white",
        "black",
        options(vec![ScatterReferenceGuide {
            kind: ScatterReferenceGuideKind::Equality,
            label: "equal rating".to_string(),
        }]),
    );
    assert_eq!(context.guides[0].start_fraction, (0.5, 1.0));
    assert_eq!(context.guides[0].end_fraction, (1.0, 0.5));
}

#[test]
fn hidden_guides_produce_no_screen_segment() {
    let context = scatter_axes_context_with_options(
        F32Range::new(0.0, 10.0),
        F32Range::new(5.0, 15.0),
        "x",
        "y",
        options(vec![ScatterReferenceGuide {
            kind: ScatterReferenceGuideKind::Horizontal { y: 0.0 },
            label: "zero".to_string(),
        }]),
    );
    assert!(context.guides.is_empty());
}

#[test]
fn timeline_axis_ticks_and_lane_labels_remain_bounded() {
    let labels = (0..8)
        .map(|lane| format!("lane-{lane}"))
        .collect::<Vec<_>>();
    let context = timeline_axes_context(U64Range::new(1_000, 2_000), 8, &labels, 5, 4);
    assert_eq!(context.time.ticks.first().unwrap().label, "1000");
    assert_eq!(context.time.ticks.last().unwrap().label, "2000");
    assert_eq!(context.lanes.len(), 4);
    assert_eq!(context.lanes.first().unwrap().label, "lane-0");
    assert_eq!(context.lanes.last().unwrap().label, "lane-7");
}

#[test]
fn timeline_ticks_preserve_u64_endpoints_without_float_rounding() {
    let min = u64::MAX - 4;
    let max = u64::MAX - 1;
    let context = timeline_axes_context(U64Range::new(min, max), 1, &[], 20, 20);

    assert!(context.time.ticks.len() <= 9);
    assert_eq!(context.time.ticks.first().unwrap().label, min.to_string());
    assert_eq!(context.time.ticks.last().unwrap().label, max.to_string());
}

#[test]
fn oversized_tick_requests_are_capped_and_adjacent_float_values_terminate() {
    let context = scatter_axes_context(
        F32Range::new(16_777_216.0, 16_777_218.0),
        F32Range::new(0.0, 1.0),
        "x",
        "y",
        usize::MAX,
    );

    assert!(context.x.ticks.len() <= 9);
    assert!(context
        .x
        .ticks
        .windows(2)
        .all(|pair| pair[0].fraction < pair[1].fraction));
}
