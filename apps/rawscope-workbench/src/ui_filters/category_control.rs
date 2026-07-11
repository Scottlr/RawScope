//! Compact categorical filtering for the workbench filter shelf.

use egui::{Align, Layout, RichText, ScrollArea, TextEdit, Ui};

use super::{FilterAction, FilterControlState};
use crate::{
    ui_dataset_identity::format_row_count,
    ui_theme::{AMBER, TEXT_MUTED},
};

const MAX_VISIBLE_CATEGORY_OPTIONS: usize = 64;
const CATEGORY_LIST_MAX_HEIGHT_PX: f32 = 148.0;

pub(super) fn show_category_filter(
    ui: &mut Ui,
    control: &FilterControlState,
) -> Option<FilterAction> {
    let FilterControlState::Categories {
        column_name,
        values,
        distinct_count,
        missing_count,
        truncated,
        include_missing,
    } = control
    else {
        return None;
    };
    let mut include_missing = *include_missing;
    let search_id = ui.id().with(("filter-search", column_name));
    let mut search = ui
        .data(|data| data.get_temp::<String>(search_id))
        .unwrap_or_default();
    if ui
        .add(
            TextEdit::singleline(&mut search)
                .hint_text("Search values")
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        ui.data_mut(|data| data.insert_temp(search_id, search.clone()));
    }

    let normalized_search = search.trim().to_lowercase();
    let mut selected = values
        .iter()
        .filter(|value| value.selected)
        .map(|value| value.value.clone())
        .collect::<Vec<_>>();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} selected", selected.len()))
                .small()
                .color(TEXT_MUTED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("None").clicked() {
                selected.clear();
                changed = true;
            }
            if ui.small_button("All").clicked() {
                selected = values.iter().map(|value| value.value.clone()).collect();
                changed = true;
            }
        });
    });

    let matching_values = values
        .iter()
        .filter(|value| {
            normalized_search.is_empty() || value.value.to_lowercase().contains(&normalized_search)
        })
        .take(MAX_VISIBLE_CATEGORY_OPTIONS)
        .collect::<Vec<_>>();
    ScrollArea::vertical()
        .max_height(CATEGORY_LIST_MAX_HEIGHT_PX)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if matching_values.is_empty() {
                ui.label(
                    RichText::new("No matching values")
                        .small()
                        .color(TEXT_MUTED),
                );
            }
            for value in matching_values {
                let mut is_selected = value.selected;
                ui.horizontal(|ui| {
                    let option_changed = ui.checkbox(&mut is_selected, "").changed();
                    ui.label(&value.value);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format_row_count(value.count))
                                .monospace()
                                .small()
                                .color(TEXT_MUTED),
                        );
                    });
                    if option_changed {
                        changed = true;
                        if is_selected {
                            selected.push(value.value.clone());
                        } else {
                            selected.retain(|candidate| candidate != &value.value);
                        }
                    }
                });
            }
        });

    if *truncated {
        ui.label(
            RichText::new(format!(
                "Showing {} retained values from {distinct_count} distinct",
                values.len()
            ))
            .small()
            .color(AMBER),
        );
    }
    if *missing_count > 0 {
        changed |= ui
            .checkbox(
                &mut include_missing,
                format!("Include {} missing", format_row_count(*missing_count)),
            )
            .changed();
    }

    changed.then(|| {
        category_selection_action(
            column_name,
            selected,
            values.len(),
            *missing_count,
            include_missing,
        )
    })
}

fn category_selection_action(
    column_name: &str,
    selected: Vec<String>,
    value_count: usize,
    missing_count: usize,
    include_missing: bool,
) -> FilterAction {
    let includes_every_value = selected.len() == value_count;
    let includes_full_domain = includes_every_value && (missing_count == 0 || include_missing);
    if includes_full_domain {
        FilterAction::RemoveColumn {
            column_name: column_name.to_string(),
        }
    } else {
        FilterAction::SetCategories {
            column_name: column_name.to_string(),
            included_values: selected,
            include_missing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::category_selection_action;
    use crate::ui_filters::FilterAction;

    #[test]
    fn selecting_the_full_category_domain_clears_the_filter() {
        let action =
            category_selection_action("winner", vec!["white".into(), "black".into()], 2, 0, false);

        assert_eq!(
            action,
            FilterAction::RemoveColumn {
                column_name: "winner".into()
            }
        );
    }

    #[test]
    fn excluding_missing_values_keeps_the_category_filter_active() {
        let action =
            category_selection_action("winner", vec!["white".into(), "black".into()], 2, 3, false);

        assert!(matches!(action, FilterAction::SetCategories { .. }));
    }
}
