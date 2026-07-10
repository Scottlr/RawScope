//! Explicit workbench interaction modes and pointer gesture ownership.

use winit::{dpi::PhysicalPosition, event::MouseButton, keyboard::KeyCode, window::CursorIcon};

use rawscope_render::{
    ScatterBrushSelection, ScatterSelectionEvidence, SelectedRegionSummary, SelectionDrilldown,
    TimelineBrushSelection, TimelineSelectionEvidence, TimelineSelectionSummary,
};

use crate::{
    app::WorkbenchApp, app_comparison::WorkbenchComparison, app_selection::ActiveLinkedSelection,
    demo::DemoMode,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WorkbenchInteractionMode {
    #[default]
    Pan,
    Brush,
    Inspect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivePointerGesture {
    Pan,
    ScatterBrush,
    TimelineBrush,
    Inspect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InteractionOverride {
    ShiftBrush,
    SecondaryButtonBrush,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct InspectCursorPosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GestureResolution {
    gesture: ActivePointerGesture,
    interaction_override: Option<InteractionOverride>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SelectionGestureBackup {
    Scatter {
        selection: Option<ScatterBrushSelection>,
        summary: Option<SelectedRegionSummary>,
        evidence: Option<ScatterSelectionEvidence>,
        drilldown: Option<SelectionDrilldown>,
        linked_selection: Option<ActiveLinkedSelection>,
        comparison: Option<WorkbenchComparison>,
    },
    Timeline {
        selection: Option<TimelineBrushSelection>,
        summary: Option<TimelineSelectionSummary>,
        evidence: Option<TimelineSelectionEvidence>,
        drilldown: Option<SelectionDrilldown>,
        linked_selection: Option<ActiveLinkedSelection>,
        comparison: Option<WorkbenchComparison>,
    },
}

impl WorkbenchApp {
    pub(crate) fn set_interaction_mode(&mut self, mode: WorkbenchInteractionMode) {
        if self.interaction_mode == mode {
            return;
        }
        self.cancel_active_pointer_gesture();
        self.clear_scatter_inspection_hover();
        self.interaction_mode = mode;
        if mode == WorkbenchInteractionMode::Inspect {
            self.refresh_inspect_cursor();
        } else {
            self.inspect_cursor_position = None;
        }
        self.update_pointer_cursor();
        self.request_redraw();
    }

    pub(crate) fn begin_pointer_gesture(&mut self, button: MouseButton) {
        if self.active_pointer_gesture.is_some() || self.cursor_fraction().is_none() {
            return;
        }
        let Some(resolution) = resolve_gesture(
            self.interaction_mode,
            button,
            self.modifiers.shift_key(),
            self.demo_mode,
        ) else {
            return;
        };
        if resolution.gesture != ActivePointerGesture::Inspect {
            self.inspect_cursor_position = None;
            self.clear_scatter_inspection_hover();
        }

        match resolution.gesture {
            ActivePointerGesture::Pan if self.demo_mode.is_scatter() => self.begin_pan(),
            ActivePointerGesture::Pan => self.begin_timeline_pan(),
            ActivePointerGesture::ScatterBrush => {
                self.capture_selection_gesture_backup();
                self.begin_brush();
            }
            ActivePointerGesture::TimelineBrush => {
                self.capture_selection_gesture_backup();
                self.begin_timeline_brush();
            }
            ActivePointerGesture::Inspect => {
                self.refresh_inspect_cursor();
                self.pin_scatter_inspection();
            }
        }

        self.active_pointer_gesture = Some(resolution.gesture);
        self.interaction_override = resolution.interaction_override;
        self.active_pointer_button = Some(button);
        self.update_pointer_cursor();
    }

    pub(crate) fn update_pointer_position(
        &mut self,
        position: PhysicalPosition<f64>,
        event_available_to_plot: bool,
    ) {
        self.cursor_position = Some(position);
        match self.active_pointer_gesture {
            Some(ActivePointerGesture::Pan) if self.demo_mode.is_scatter() => {
                self.pan_to_cursor(position)
            }
            Some(ActivePointerGesture::Pan) => self.pan_timeline_to_cursor(position),
            Some(ActivePointerGesture::ScatterBrush) => self.update_brush_to_cursor(position),
            Some(ActivePointerGesture::TimelineBrush) => {
                self.update_timeline_brush_to_cursor(position)
            }
            Some(ActivePointerGesture::Inspect) => self.refresh_inspect_cursor(),
            None if event_available_to_plot
                && self.interaction_mode == WorkbenchInteractionMode::Inspect =>
            {
                self.refresh_inspect_cursor()
            }
            None => {
                self.inspect_cursor_position = None;
                self.clear_scatter_inspection_hover();
            }
        }
        self.update_pointer_cursor();
    }

    pub(crate) fn end_pointer_gesture(&mut self, button: MouseButton) {
        if self.active_pointer_button != Some(button) {
            if self.interaction_mode == WorkbenchInteractionMode::Inspect
                && button == MouseButton::Left
                && self.cursor_fraction().is_some()
            {
                self.pin_scatter_inspection();
            }
            return;
        }
        match self.active_pointer_gesture {
            Some(ActivePointerGesture::Pan) => self.end_pan(),
            Some(ActivePointerGesture::ScatterBrush) => self.end_brush(),
            Some(ActivePointerGesture::TimelineBrush) => self.end_timeline_brush(),
            Some(ActivePointerGesture::Inspect) | None => {}
        }
        self.selection_gesture_backup = None;
        self.clear_pointer_ownership();
        if self.interaction_mode == WorkbenchInteractionMode::Inspect {
            self.refresh_inspect_cursor();
        }
        self.update_pointer_cursor();
    }

    pub(crate) fn handle_escape(&mut self) {
        if self.active_pointer_gesture.is_some() {
            self.cancel_active_pointer_gesture();
            return;
        }
        match self.demo_mode {
            DemoMode::Scatter => self.clear_brush(),
            DemoMode::Timeline => self.clear_timeline_brush(),
        }
    }

    pub(crate) fn cancel_active_pointer_gesture(&mut self) {
        match self.active_pointer_gesture {
            Some(ActivePointerGesture::Pan) => self.end_pan(),
            Some(ActivePointerGesture::ScatterBrush) => self.cancel_brush_gesture(),
            Some(ActivePointerGesture::TimelineBrush) => self.cancel_timeline_brush_gesture(),
            Some(ActivePointerGesture::Inspect) | None => {}
        }
        self.restore_selection_gesture_backup();
        self.clear_pointer_ownership();
        self.update_pointer_cursor();
        self.request_redraw();
    }

    pub(crate) fn update_pointer_cursor(&self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let pointer_is_in_plot = self.cursor_fraction().is_some();
        window.set_cursor(cursor_icon(
            self.interaction_mode,
            self.active_pointer_gesture,
            pointer_is_in_plot,
        ));
    }

    pub(crate) fn reassert_plot_cursor(&self) {
        if self.active_pointer_gesture.is_some() || self.cursor_fraction().is_some() {
            self.update_pointer_cursor();
        }
    }

    fn clear_pointer_ownership(&mut self) {
        self.active_pointer_gesture = None;
        self.interaction_override = None;
        self.active_pointer_button = None;
    }

    fn capture_selection_gesture_backup(&mut self) {
        self.selection_gesture_backup = Some(match self.demo_mode {
            DemoMode::Scatter => SelectionGestureBackup::Scatter {
                selection: self.scatter.active_brush_selection,
                summary: self.scatter.selection_summary,
                evidence: self.scatter.selection_evidence.clone(),
                drilldown: self.scatter.selection_drilldown.clone(),
                linked_selection: self.active_selection.clone(),
                comparison: self.active_comparison.clone(),
            },
            DemoMode::Timeline => SelectionGestureBackup::Timeline {
                selection: self.timeline.active_brush_selection,
                summary: self.timeline.selection_summary.clone(),
                evidence: self.timeline.selection_evidence.clone(),
                drilldown: self.timeline.selection_drilldown.clone(),
                linked_selection: self.active_selection.clone(),
                comparison: self.active_comparison.clone(),
            },
        });
    }

    fn restore_selection_gesture_backup(&mut self) {
        match self.selection_gesture_backup.take() {
            Some(SelectionGestureBackup::Scatter {
                selection,
                summary,
                evidence,
                drilldown,
                linked_selection,
                comparison,
            }) => {
                self.scatter.active_brush_selection = selection;
                self.scatter.selection_summary = summary;
                self.scatter.selection_evidence = evidence;
                self.scatter.selection_drilldown = drilldown;
                self.active_selection = linked_selection;
                self.active_comparison = comparison;
            }
            Some(SelectionGestureBackup::Timeline {
                selection,
                summary,
                evidence,
                drilldown,
                linked_selection,
                comparison,
            }) => {
                self.timeline.active_brush_selection = selection;
                self.timeline.selection_summary = summary;
                self.timeline.selection_evidence = evidence;
                self.timeline.selection_drilldown = drilldown;
                self.active_selection = linked_selection;
                self.active_comparison = comparison;
            }
            None => {}
        }
        self.update_window_title();
    }

    fn refresh_inspect_cursor(&mut self) {
        let Some((x_fraction, y_fraction)) = self.cursor_fraction() else {
            self.inspect_cursor_position = None;
            return;
        };
        self.inspect_cursor_position = match self.demo_mode {
            DemoMode::Scatter => self.scatter.viewport.map(|viewport| {
                let (x, y) = viewport.data_point_at_fraction(x_fraction, y_fraction);
                InspectCursorPosition { x, y }
            }),
            DemoMode::Timeline => self
                .timeline
                .viewport
                .map(|viewport| InspectCursorPosition {
                    x: viewport.time_at_fraction(x_fraction) as f32,
                    y: y_fraction * viewport.lane_count() as f32,
                }),
        };
        if self.demo_mode == DemoMode::Scatter {
            self.refresh_scatter_inspection_hover();
        }
        self.request_redraw();
    }
}

pub(crate) fn interaction_mode_for_shortcut(key: KeyCode) -> Option<WorkbenchInteractionMode> {
    match key {
        KeyCode::KeyH => Some(WorkbenchInteractionMode::Pan),
        KeyCode::KeyB => Some(WorkbenchInteractionMode::Brush),
        KeyCode::KeyI => Some(WorkbenchInteractionMode::Inspect),
        _ => None,
    }
}

fn resolve_gesture(
    persistent_mode: WorkbenchInteractionMode,
    button: MouseButton,
    shift_pressed: bool,
    demo_mode: DemoMode,
) -> Option<GestureResolution> {
    let brush_gesture = if demo_mode.is_scatter() {
        ActivePointerGesture::ScatterBrush
    } else {
        ActivePointerGesture::TimelineBrush
    };
    if button == MouseButton::Right {
        return Some(GestureResolution {
            gesture: brush_gesture,
            interaction_override: Some(InteractionOverride::SecondaryButtonBrush),
        });
    }
    if button == MouseButton::Left && shift_pressed {
        return Some(GestureResolution {
            gesture: brush_gesture,
            interaction_override: Some(InteractionOverride::ShiftBrush),
        });
    }
    if button == MouseButton::Middle {
        return Some(GestureResolution {
            gesture: ActivePointerGesture::Pan,
            interaction_override: None,
        });
    }
    if button != MouseButton::Left {
        return None;
    }

    let gesture = match persistent_mode {
        WorkbenchInteractionMode::Pan => ActivePointerGesture::Pan,
        WorkbenchInteractionMode::Brush => brush_gesture,
        WorkbenchInteractionMode::Inspect => ActivePointerGesture::Inspect,
    };
    Some(GestureResolution {
        gesture,
        interaction_override: None,
    })
}

fn cursor_icon(
    persistent_mode: WorkbenchInteractionMode,
    active_gesture: Option<ActivePointerGesture>,
    pointer_is_in_plot: bool,
) -> CursorIcon {
    match active_gesture {
        Some(ActivePointerGesture::Pan) => CursorIcon::Grabbing,
        Some(ActivePointerGesture::ScatterBrush | ActivePointerGesture::TimelineBrush)
        | Some(ActivePointerGesture::Inspect) => CursorIcon::Crosshair,
        None if pointer_is_in_plot && persistent_mode == WorkbenchInteractionMode::Pan => {
            CursorIcon::Grab
        }
        None if pointer_is_in_plot => CursorIcon::Crosshair,
        None => CursorIcon::Default,
    }
}

#[cfg(test)]
#[path = "app_interaction_mode_tests.rs"]
mod tests;
