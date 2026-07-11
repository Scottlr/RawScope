//! Filter shelf composition and numeric range controls.

use egui::{
    collapsing_header::CollapsingState, Align, DragValue, Layout, ProgressBar, RichText, Ui,
};

use super::{
    category_control::show_category_filter, FilterAction, FilterControlState, ScatterFiltersUiState,
};
use crate::{
    ui_dataset_identity::format_row_count,
    ui_theme::{
        compact_icon_button, error_callout_frame, filter_card_frame, filter_chip_frame, ACCENT,
        AMBER, TEXT_MUTED, TEXT_PRIMARY,
    },
};

pub(crate) fn show_filters(
    ui: &mut Ui,
    state: Option<&ScatterFiltersUiState>,
) -> Option<FilterAction> {
    let state = state?;
    let mut action = None;
    ui.horizontal(|ui| {
        ui.heading("Filters");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !state.active_columns.is_empty() && ui.small_button("Clear all").clicked() {
                action = Some(FilterAction::ClearAll);
            }
        });
    });
    show_cohort_summary(ui, state);

    if !state.active_columns.is_empty() {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for column in &state.active_columns {
                filter_chip_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(field_display_name(column))
                                .small()
                                .color(TEXT_PRIMARY),
                        );
                        if compact_icon_button(ui, "x", &format!("Clear {column} filter")).clicked()
                        {
                            action = Some(FilterAction::RemoveColumn {
                                column_name: column.clone(),
                            });
                        }
                    });
                });
            }
        });
    }
    if let Some(error) = &state.error {
        ui.add_space(4.0);
        error_callout_frame().show(ui, |ui| {
            ui.label(RichText::new(error).small());
        });
    }

    ui.add_space(8.0);
    for (control_index, control) in state.controls.iter().enumerate() {
        let active = state
            .active_columns
            .iter()
            .any(|column| column == control.column_name());
        if let Some(next) = show_filter_card(ui, control, active, control_index == 0) {
            action = Some(next);
        }
        ui.add_space(6.0);
    }

    if state.available_columns.is_empty() {
        ui.label(
            RichText::new("All available fields are shown")
                .small()
                .color(TEXT_MUTED),
        );
    } else {
        ui.menu_button("Add filter field", |ui| {
            for column in &state.available_columns {
                if ui.button(field_display_name(column)).clicked() {
                    action = Some(FilterAction::AddColumn {
                        column_name: column.clone(),
                    });
                    ui.close();
                }
            }
        });
    }
    action
}

fn show_cohort_summary(ui: &mut Ui, state: &ScatterFiltersUiState) {
    let cohort_ratio = if state.total_count == 0 {
        0.0
    } else {
        state.included_count as f32 / state.total_count as f32
    };
    let summary_color = if state.active_columns.is_empty() {
        TEXT_MUTED
    } else {
        ACCENT
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("ACTIVE COHORT").small().color(TEXT_MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!(
                    "{} of {}",
                    format_row_count(state.included_count),
                    format_row_count(state.total_count)
                ))
                .small()
                .strong()
                .color(summary_color),
            );
        });
    });
    ui.add(
        ProgressBar::new(cohort_ratio)
            .desired_height(5.0)
            .fill(summary_color),
    );
}

fn show_filter_card(
    ui: &mut Ui,
    control: &FilterControlState,
    active: bool,
    default_open: bool,
) -> Option<FilterAction> {
    filter_card_frame(active)
        .show(ui, |ui| {
            let column_name = control.column_name();
            let collapse_id = ui.make_persistent_id(("filter-card", column_name));
            let (_, _, body) = CollapsingState::load_with_default_open(
                ui.ctx(),
                collapse_id,
                default_open || active,
            )
            .show_header(ui, |ui| {
                ui.label(
                    RichText::new(field_display_name(column_name))
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(control_summary(control, active))
                            .small()
                            .color(if active { ACCENT } else { TEXT_MUTED }),
                    );
                    if active {
                        ui.label(RichText::new("ACTIVE").small().strong().color(ACCENT));
                    }
                });
            })
            .body_unindented(|ui| {
                ui.add_space(8.0);
                match control {
                    FilterControlState::NumericRange { .. } => show_numeric_filter(ui, control),
                    FilterControlState::Categories { .. } => show_category_filter(ui, control),
                }
            });
            body.and_then(|response| response.inner)
        })
        .inner
}

fn control_summary(control: &FilterControlState, active: bool) -> String {
    match control {
        FilterControlState::NumericRange {
            selected_min,
            selected_max,
            ..
        } if active => format!(
            "{} to {}",
            format_filter_number(*selected_min),
            format_filter_number(*selected_max)
        ),
        FilterControlState::NumericRange { .. } => "Numeric range".to_string(),
        FilterControlState::Categories {
            values,
            distinct_count,
            ..
        } if active => format!(
            "{} of {} values",
            values.iter().filter(|value| value.selected).count(),
            distinct_count
        ),
        FilterControlState::Categories { distinct_count, .. } => {
            format!("{distinct_count} values")
        }
    }
}

fn show_numeric_filter(ui: &mut Ui, control: &FilterControlState) -> Option<FilterAction> {
    let FilterControlState::NumericRange {
        column_name,
        domain_min,
        domain_max,
        selected_min,
        selected_max,
        missing_count,
        invalid_count,
        include_missing,
    } = control
    else {
        return None;
    };
    let (domain_min, domain_max) = (*domain_min, *domain_max);
    let (mut selected_min, mut selected_max) = (*selected_min, *selected_max);
    let mut include_missing = *include_missing;
    ui.label(
        RichText::new(format!(
            "Available {} to {}",
            format_filter_number(domain_min),
            format_filter_number(domain_max)
        ))
        .small()
        .color(TEXT_MUTED),
    );
    let drag_speed = ((domain_max - domain_min).abs() / 200.0).max(f64::EPSILON);
    let (min_changed, max_changed) = ui
        .horizontal(|ui| {
            ui.label(RichText::new("From").small().color(TEXT_MUTED));
            let min_changed = ui
                .add_sized(
                    [84.0, 28.0],
                    DragValue::new(&mut selected_min)
                        .range(domain_min..=domain_max)
                        .speed(drag_speed)
                        .max_decimals(3),
                )
                .changed();
            ui.label(RichText::new("to").small().color(TEXT_MUTED));
            let max_changed = ui
                .add_sized(
                    [84.0, 28.0],
                    DragValue::new(&mut selected_max)
                        .range(domain_min..=domain_max)
                        .speed(drag_speed)
                        .max_decimals(3),
                )
                .changed();
            (min_changed, max_changed)
        })
        .inner;
    let missing_changed = *missing_count > 0
        && ui
            .checkbox(
                &mut include_missing,
                format!("Include {} missing", format_row_count(*missing_count)),
            )
            .changed();
    if *invalid_count > 0 {
        ui.label(
            RichText::new(format!(
                "{} invalid values are excluded",
                format_row_count(*invalid_count)
            ))
            .small()
            .color(AMBER),
        );
    }
    (min_changed || max_changed || missing_changed).then(|| FilterAction::SetNumericRange {
        column_name: column_name.to_string(),
        min_inclusive: selected_min.min(selected_max),
        max_inclusive: selected_min.max(selected_max),
        include_missing,
    })
}

fn field_display_name(column_name: &str) -> String {
    let mut words = column_name.split('_').filter(|word| !word.is_empty());
    let Some(first) = words.next() else {
        return column_name.to_string();
    };
    let mut display = first.to_string();
    if let Some(initial) = display.get_mut(0..1) {
        initial.make_ascii_uppercase();
    }
    for word in words {
        display.push(' ');
        display.push_str(word);
    }
    display
}

fn format_filter_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{field_display_name, format_filter_number};

    #[test]
    fn filter_labels_are_readable_without_changing_source_names() {
        assert_eq!(field_display_name("time_control"), "Time control");
        assert_eq!(field_display_name("winner"), "Winner");
    }

    #[test]
    fn numeric_filter_summary_avoids_noisy_trailing_zeroes() {
        assert_eq!(format_filter_number(800.0), "800");
        assert_eq!(format_filter_number(0.125), "0.125");
        assert_eq!(format_filter_number(2.50), "2.5");
    }
}
