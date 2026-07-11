//! RawScope-owned visual language for the egui workbench shell.

use std::{collections::BTreeMap, sync::Arc};

use egui::{
    vec2, Button, Color32, Context, CornerRadius, CursorIcon, FontData, FontDefinitions,
    FontFamily, FontId, Frame, Margin, Response, RichText, Shadow, Stroke, Style, TextStyle, Ui,
    Visuals,
};
use iconflow::{Pack, Size, Style as IconStyle};

pub(crate) const ACCENT: Color32 = Color32::from_rgb(63, 191, 184);
pub(crate) const ACCENT_BRIGHT: Color32 = Color32::from_rgb(87, 216, 205);
pub(crate) const AMBER: Color32 = Color32::from_rgb(238, 174, 68);
pub(crate) const TEXT_PRIMARY: Color32 = Color32::from_rgb(226, 232, 232);
pub(crate) const TEXT_MUTED: Color32 = Color32::from_rgb(145, 158, 158);

const APP_BACKGROUND: Color32 = Color32::from_rgb(14, 17, 18);
const PANEL_BACKGROUND: Color32 = Color32::from_rgb(21, 25, 27);
const PANEL_RAISED: Color32 = Color32::from_rgb(28, 33, 35);
const PANEL_HOVERED: Color32 = Color32::from_rgb(35, 42, 44);
const BORDER: Color32 = Color32::from_rgb(53, 63, 65);
const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 46, 48);
const SELECTION_BACKGROUND: Color32 = Color32::from_rgb(30, 102, 99);
const FILTER_ACTIVE_BACKGROUND: Color32 = Color32::from_rgb(23, 39, 39);
const ERROR: Color32 = Color32::from_rgb(232, 96, 101);
const ERROR_BACKGROUND: Color32 = Color32::from_rgb(49, 27, 30);
const SUCCESS: Color32 = Color32::from_rgb(102, 194, 132);

const CONTROL_CORNER_RADIUS_PX: u8 = 4;
const PANEL_HORIZONTAL_MARGIN_PX: i8 = 14;
const PANEL_VERTICAL_MARGIN_PX: i8 = 10;

pub(crate) fn apply_theme(context: &Context) {
    register_icon_fonts(context);
    let mut style = Style {
        text_styles: BTreeMap::from([
            (
                TextStyle::Heading,
                FontId::new(17.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(12.0, FontFamily::Monospace),
            ),
            (
                TextStyle::Button,
                FontId::new(12.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(11.0, FontFamily::Proportional),
            ),
        ]),
        visuals: rawscope_visuals(),
        ..Style::default()
    };

    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(10.0, 6.0);
    style.spacing.interact_size = vec2(28.0, 28.0);
    style.spacing.window_margin = Margin::symmetric(12, 10);
    style.spacing.menu_margin = Margin::symmetric(8, 6);
    style.spacing.indent = 16.0;
    style.animation_time = 0.12;
    style.compact_menu_style = true;

    context.set_global_style(style);
}

fn register_icon_fonts(context: &Context) {
    let mut definitions = FontDefinitions::default();
    for font in iconflow::fonts() {
        let font_name = font.family.to_string();
        definitions.font_data.insert(
            font_name.clone(),
            Arc::new(FontData::from_static(font.bytes)),
        );
        definitions
            .families
            .insert(FontFamily::Name(font.family.into()), vec![font_name]);
    }
    context.set_fonts(definitions);
}

pub(crate) fn toolbar_frame() -> Frame {
    Frame::new()
        .fill(APP_BACKGROUND)
        .inner_margin(Margin::symmetric(PANEL_HORIZONTAL_MARGIN_PX, 9))
        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
}

pub(crate) fn right_rail_frame() -> Frame {
    Frame::new()
        .fill(PANEL_BACKGROUND)
        .inner_margin(Margin::symmetric(
            PANEL_HORIZONTAL_MARGIN_PX,
            PANEL_VERTICAL_MARGIN_PX,
        ))
        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
}

pub(crate) fn status_bar_frame() -> Frame {
    Frame::new()
        .fill(APP_BACKGROUND)
        .inner_margin(Margin::symmetric(PANEL_HORIZONTAL_MARGIN_PX, 5))
        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
}

pub(crate) fn filter_card_frame(active: bool) -> Frame {
    Frame::new()
        .fill(if active {
            FILTER_ACTIVE_BACKGROUND
        } else {
            PANEL_RAISED
        })
        .corner_radius(CONTROL_CORNER_RADIUS_PX)
        .inner_margin(Margin::symmetric(10, 8))
        .stroke(Stroke::new(
            1.0,
            if active { ACCENT } else { BORDER_SUBTLE },
        ))
}

pub(crate) fn filter_chip_frame() -> Frame {
    Frame::new()
        .fill(FILTER_ACTIVE_BACKGROUND)
        .corner_radius(CONTROL_CORNER_RADIUS_PX)
        .inner_margin(Margin::symmetric(7, 2))
        .stroke(Stroke::new(1.0, Color32::from_rgb(39, 79, 77)))
}

pub(crate) fn error_callout_frame() -> Frame {
    Frame::new()
        .fill(ERROR_BACKGROUND)
        .corner_radius(CONTROL_CORNER_RADIUS_PX)
        .inner_margin(Margin::symmetric(8, 6))
        .stroke(Stroke::new(1.0, ERROR))
}

pub(crate) fn navigation_button(label: &str, selected: bool) -> Button<'_> {
    Button::new(RichText::new(label).strong())
        .selected(selected)
        .min_size(vec2(82.0, 30.0))
        .corner_radius(CONTROL_CORNER_RADIUS_PX)
}

pub(crate) fn icon_command_button(
    ui: &mut Ui,
    icon_name: &str,
    tooltip: &str,
    enabled: bool,
) -> Response {
    ui.add_enabled(
        enabled,
        Button::new(icon_text(icon_name, 16.0))
            .min_size(vec2(30.0, 30.0))
            .corner_radius(CONTROL_CORNER_RADIUS_PX),
    )
    .on_hover_text(tooltip)
}

pub(crate) fn compact_icon_button(ui: &mut Ui, icon_name: &str, tooltip: &str) -> Response {
    ui.add(
        Button::new(icon_text(icon_name, 13.0))
            .min_size(vec2(22.0, 22.0))
            .corner_radius(CONTROL_CORNER_RADIUS_PX),
    )
    .on_hover_text(tooltip)
}

pub(crate) fn icon_segment_button(
    ui: &mut Ui,
    icon_name: &str,
    tooltip: &str,
    selected: bool,
) -> Response {
    ui.add(
        Button::new(icon_text(icon_name, 16.0))
            .selected(selected)
            .min_size(vec2(32.0, 30.0))
            .corner_radius(CONTROL_CORNER_RADIUS_PX),
    )
    .on_hover_text(tooltip)
}

fn icon_text(icon_name: &str, size_px: f32) -> RichText {
    let icon = iconflow::try_icon(Pack::Lucide, icon_name, IconStyle::Regular, Size::Regular)
        .expect("workbench icons are compile-time constants");
    let glyph = char::from_u32(icon.codepoint).expect("iconflow returns valid Unicode codepoints");
    RichText::new(glyph.to_string())
        .font(FontId::new(size_px, FontFamily::Name(icon.family.into())))
}

pub(crate) fn segmented_button(label: &str, selected: bool) -> Button<'_> {
    Button::new(label)
        .selected(selected)
        .min_size(vec2(98.0, 26.0))
        .corner_radius(CONTROL_CORNER_RADIUS_PX)
}

pub(crate) fn status_badge(ui: &mut Ui, label: &str, active: bool) {
    let fill = if active {
        Color32::from_rgb(25, 67, 64)
    } else {
        PANEL_RAISED
    };
    let text_color = if active { ACCENT_BRIGHT } else { TEXT_MUTED };
    Frame::new()
        .fill(fill)
        .corner_radius(CONTROL_CORNER_RADIUS_PX)
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().color(text_color));
        });
}

pub(crate) fn export_status_color(status_is_error: bool, status_is_complete: bool) -> Color32 {
    if status_is_error {
        ERROR
    } else if status_is_complete {
        SUCCESS
    } else {
        TEXT_MUTED
    }
}

fn rawscope_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.weak_text_color = Some(TEXT_MUTED);
    visuals.panel_fill = PANEL_BACKGROUND;
    visuals.window_fill = PANEL_BACKGROUND;
    visuals.extreme_bg_color = APP_BACKGROUND;
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.code_bg_color = APP_BACKGROUND;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(CONTROL_CORNER_RADIUS_PX);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 1,
        color: Color32::from_black_alpha(120),
    };
    visuals.selection.bg_fill = SELECTION_BACKGROUND;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.hyperlink_color = ACCENT_BRIGHT;
    visuals.warn_fg_color = AMBER;
    visuals.error_fg_color = ERROR;
    visuals.striped = true;
    visuals.interact_cursor = Some(CursorIcon::PointingHand);
    visuals.disabled_alpha = 0.42;

    let corner_radius = CornerRadius::same(CONTROL_CORNER_RADIUS_PX);
    visuals.widgets.noninteractive.bg_fill = PANEL_BACKGROUND;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL_BACKGROUND;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.noninteractive.corner_radius = corner_radius;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.weak_bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.corner_radius = corner_radius;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = PANEL_HOVERED;
    visuals.widgets.hovered.weak_bg_fill = PANEL_HOVERED;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.corner_radius = corner_radius;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = SELECTION_BACKGROUND;
    visuals.widgets.active.weak_bg_fill = SELECTION_BACKGROUND;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.active.corner_radius = corner_radius;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.expansion = 0.0;
    visuals.widgets.open = visuals.widgets.active;

    visuals
}
