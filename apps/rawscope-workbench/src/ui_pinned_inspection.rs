//! Durable pinned inspection hierarchy for the workbench right rail.

use egui::{RichText, Ui};
use rawscope_render::{format_axis_value, AxisValueFormat, DifferenceDirection};

use crate::{
    app_scatter_inspection::{PinnedCategorySummary, PinnedScatterInspection},
    ui_scatter_inspection::{ScatterInspectionAction, ScatterInspectionUiState},
    ui_theme::{compact_icon_button, ACCENT, TEXT_MUTED},
};

pub(crate) fn show_pinned_scatter_inspection(
    ui: &mut Ui,
    state: Option<&ScatterInspectionUiState>,
) -> Option<ScatterInspectionAction> {
    let state = state?;
    let pinned = state.pinned.as_ref()?;
    let mut action = None;

    ui.horizontal(|ui| {
        ui.label(RichText::new("Pinned region").strong().color(ACCENT));
        if compact_icon_button(ui, "x", "Clear pinned inspection").clicked() {
            action = Some(ScatterInspectionAction::ClearPinned);
        }
    });
    ui.add_space(4.0);
    show_exact_facts(ui, state, pinned);
    ui.add_space(4.0);
    show_sample_facts(ui, pinned);
    action
}

fn show_exact_facts(
    ui: &mut Ui,
    state: &ScatterInspectionUiState,
    pinned: &PinnedScatterInspection,
) {
    let summary = &pinned.summary;
    let hit = &summary.hit;
    ui.label(RichText::new("Exact bin").small().color(TEXT_MUTED));
    ui.label(format!(
        "{} {} .. {}",
        state.x_label,
        format_axis_value(hit.x_range.min, AxisValueFormat::Compact),
        format_axis_value(hit.x_range.max, AxisValueFormat::Compact)
    ));
    ui.label(format!(
        "{} {} .. {}",
        state.y_label,
        format_axis_value(hit.y_range.min, AxisValueFormat::Compact),
        format_axis_value(hit.y_range.max, AxisValueFormat::Compact)
    ));
    ui.label(RichText::new(format!("{} rows", hit.count)).strong());
    ui.label(format!(
        "{:.2}% of active rows",
        summary.active_share * 100.0
    ));
    if let Some(percentile) = summary.occupied_density_percentile {
        ui.label(format!(
            "{:.1}th percentile among occupied cells",
            percentile * 100.0
        ));
    }
    ui.label(
        RichText::new(format!(
            "Nearby 3x3: {} rows ({:.2}% of active)",
            summary.neighborhood_count,
            summary.neighborhood_share * 100.0
        ))
        .small()
        .color(TEXT_MUTED),
    );
    if let Some(difference) = pinned.difference.as_ref() {
        ui.add_space(3.0);
        ui.label(RichText::new("Difference").small().color(TEXT_MUTED));
        ui.label(format!(
            "Active {:.3}% | full baseline {:.3}%",
            difference.active_share * 100.0,
            difference.baseline_share * 100.0
        ));
        ui.label(format!(
            "Counts: {} active / {} baseline ({} / {} total)",
            difference.active_count,
            difference.baseline_count,
            difference.active_total,
            difference.baseline_total
        ));
        ui.label(format!(
            "Support context: {:.3}% (presentation aid, not significance)",
            difference.support_share * 100.0
        ));
        let direction = match difference.direction {
            DifferenceDirection::MoreCommonInActive => "More common in active",
            DifferenceDirection::LessCommonInActive => "Less common in active",
            DifferenceDirection::Unchanged => "Unchanged",
        };
        ui.label(format!(
            "{direction}: {:+.3} percentage points",
            difference.share_delta * 100.0
        ));
        if let Some(percentile) = difference.absolute_delta_percentile {
            ui.label(format!(
                "Difference strength: {:.1}th percentile",
                percentile * 100.0
            ));
        }
    }
}

fn show_sample_facts(ui: &mut Ui, pinned: &PinnedScatterInspection) {
    let hit = &pinned.summary.hit;
    let sample_count = pinned.source_rows.len();
    if sample_count > 0 {
        ui.label(RichText::new(format!(
            "Sampled evidence: {sample_count} of {} bin rows",
            hit.count
        )));
    }
    for summary in &pinned.category_summaries {
        show_category_summary(ui, summary);
    }
    if !pinned.evidence_keys.is_empty() {
        ui.label(
            RichText::new("Evidence-key sample")
                .small()
                .color(TEXT_MUTED),
        );
        for key in &pinned.evidence_keys {
            let display_value = elide_value(&key.value, 42);
            ui.monospace(format!("{}: {display_value}", key.column_name))
                .on_hover_text(&key.value);
        }
    }
    if !pinned.source_rows.is_empty() {
        ui.label(
            RichText::new(
                pinned
                    .source_rows
                    .iter()
                    .map(|row| row.row_id.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .monospace()
            .small()
            .color(TEXT_MUTED),
        );
    }
}

fn show_category_summary(ui: &mut Ui, summary: &PinnedCategorySummary) {
    let values = summary
        .value_counts
        .iter()
        .map(|(value, count)| format!("{value} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    ui.label(
        RichText::new(format!(
            "Sample categories ({} of {} rows) | {}: {values}",
            summary.sample_row_count, summary.bin_row_count, summary.column_name
        ))
        .small()
        .color(TEXT_MUTED),
    );
}

fn elide_value(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_categories_disclose_sample_size() {
        let summary = PinnedCategorySummary {
            column_name: "winner".into(),
            sample_row_count: 16,
            bin_row_count: 842,
            value_counts: vec![("white".into(), 9)],
        };

        assert_eq!(elide_value("abcdefgh", 4), "abcd...");
        assert_eq!(summary.sample_row_count, 16);
        assert_eq!(summary.bin_row_count, 842);
    }
}
