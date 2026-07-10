//! Compact selection-vs-baseline comparison panel in the right rail.

use std::cmp::Ordering;

use egui::{RichText, Ui};
use rawscope_render::{
    ComparisonRatio, MissingnessSelectionComparison, ScatterSelectionComparison,
    TimelineSelectionComparison,
};

use crate::app_comparison::WorkbenchComparison;

const COMPARISON_TOP_DELTA_LIMIT: usize = 3;

pub(crate) fn show_selection_comparison(
    ui: &mut Ui,
    comparison: Option<&WorkbenchComparison>,
) -> bool {
    let Some(comparison) = comparison else {
        return false;
    };

    ui.heading("Selection Comparison");
    match comparison {
        WorkbenchComparison::Scatter(comparison) => {
            show_scatter_comparison(ui, comparison);
        }
        WorkbenchComparison::Timeline(comparison) => {
            show_timeline_comparison(ui, comparison);
        }
        WorkbenchComparison::Missingness(comparison) => {
            show_missingness_comparison(ui, comparison);
        }
    }

    true
}

fn show_scatter_comparison(ui: &mut Ui, comparison: &ScatterSelectionComparison) {
    ui.label(format!(
        "Selected rows: {} baseline {} ({:.1}%)",
        comparison.selected_row_count,
        comparison.baseline_row_count,
        comparison.selected_percentage
    ));
    show_top_delta_lines(
        ui,
        &[
            named_ratio("cluster", comparison.point_kind_ratios.cluster),
            named_ratio("background", comparison.point_kind_ratios.background),
            named_ratio("outlier", comparison.point_kind_ratios.outlier),
            named_ratio("unclassified", comparison.point_kind_ratios.unclassified),
        ],
    );
}

fn show_timeline_comparison(ui: &mut Ui, comparison: &TimelineSelectionComparison) {
    ui.label(format!(
        "Selected events: {} baseline {} ({:.1}%)",
        comparison.selected_event_count,
        comparison.baseline_event_count,
        comparison.selected_percentage
    ));
    let mut ratios = Vec::with_capacity(5 + comparison.lane_ratios.len());

    ratios.push(named_ratio(
        "background",
        comparison.event_kind_ratios.background,
    ));
    ratios.push(named_ratio("spike", comparison.event_kind_ratios.spike));
    ratios.push(named_ratio(
        "stale lane",
        comparison.event_kind_ratios.stale_lane,
    ));
    ratios.push(named_ratio(
        "high value band",
        comparison.event_kind_ratios.high_value_band,
    ));
    ratios.push(named_ratio(
        "unclassified",
        comparison.event_kind_ratios.unclassified,
    ));
    for (lane_index, ratio) in comparison.lane_ratios.iter().enumerate() {
        ratios.push(named_ratio(format!("lane {lane_index}"), *ratio));
    }

    show_top_delta_lines(ui, &ratios);
}

fn show_missingness_comparison(ui: &mut Ui, comparison: &MissingnessSelectionComparison) {
    ui.label(format!(
        "Selected missing: {} of {} ({:.1}%)",
        comparison.selected_missing_count,
        comparison.selected_total_count,
        comparison.selected_missing_ratio * 100.0
    ));
    ui.label(format!(
        "Baseline missing: {} of {} ({:.1}%)",
        comparison.baseline_missing_count,
        comparison.baseline_total_count,
        comparison.baseline_missing_ratio * 100.0
    ));
    ui.label(format!(
        "Delta: {:+.1} percentage points",
        comparison.delta_percentage_points
    ));
}

fn show_top_delta_lines(ui: &mut Ui, ratios: &[NamedComparisonRatio]) {
    let mut top_deltas = ratios.to_vec();
    top_deltas.sort_by(|a, b| {
        let a_delta = abs_delta(a.ratio.delta_percentage_points);
        let b_delta = abs_delta(b.ratio.delta_percentage_points);
        b_delta.partial_cmp(&a_delta).unwrap_or(Ordering::Equal)
    });

    if top_deltas.is_empty() {
        return;
    }

    ui.separator();
    ui.label(RichText::new("Top delta lines").small());
    for ratio in top_deltas.into_iter().take(COMPARISON_TOP_DELTA_LIMIT) {
        ui.label(format!(
            "{:<16} sel {:>4.1}% baseline {:>5.1}% delta {:+.1}pp",
            ratio.name,
            ratio.ratio.selected_percentage,
            ratio.ratio.baseline_percentage,
            ratio.ratio.delta_percentage_points
        ));
    }
}

fn named_ratio(name: impl ToString, ratio: ComparisonRatio) -> NamedComparisonRatio {
    NamedComparisonRatio {
        name: name.to_string(),
        ratio,
    }
}

fn abs_delta(value: f32) -> f32 {
    value.abs()
}

#[derive(Debug, Clone)]
struct NamedComparisonRatio {
    name: String,
    ratio: ComparisonRatio,
}
