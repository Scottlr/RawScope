use egui::{pos2, Rect};
use rawscope_core::F32Range;
use rawscope_render::{PlotRectPx, ScatterViewport};

use super::*;
use crate::{
    ui_plot_surface::{PlotAxisLayout, PlotSurfaceLayout},
    WorkbenchApp,
};

fn scatter_app_with_plot() -> WorkbenchApp {
    WorkbenchApp {
        demo_mode: DemoMode::Scatter,
        plot_surface: Some(PlotSurfaceLayout {
            logical_rect: Rect::from_min_max(pos2(50.0, 25.0), pos2(250.0, 125.0)),
            physical_rect: PlotRectPx::try_new(100, 50, 400, 200, 800, 600).unwrap(),
            axis_layout: PlotAxisLayout {
                outer_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(262.0, 163.0)),
                plot_rect: Rect::from_min_max(pos2(50.0, 25.0), pos2(250.0, 125.0)),
            },
        }),
        scatter: crate::app::ScatterWorkbenchState {
            viewport: Some(ScatterViewport::new(
                F32Range::new(0.0, 100.0),
                F32Range::new(0.0, 200.0),
            )),
            ..crate::app::ScatterWorkbenchState::default()
        },
        ..WorkbenchApp::default()
    }
}

#[test]
fn pan_is_the_default_persistent_mode() {
    assert_eq!(
        WorkbenchApp::default().interaction_mode,
        WorkbenchInteractionMode::Pan
    );
}

#[test]
fn shift_left_temporarily_overrides_pan_with_brush() {
    let resolution = resolve_gesture(
        WorkbenchInteractionMode::Pan,
        MouseButton::Left,
        true,
        DemoMode::Scatter,
    )
    .unwrap();

    assert_eq!(resolution.gesture, ActivePointerGesture::ScatterBrush);
    assert_eq!(
        resolution.interaction_override,
        Some(InteractionOverride::ShiftBrush)
    );
}

#[test]
fn secondary_brush_release_restores_persistent_mode() {
    let mut app = scatter_app_with_plot();
    app.cursor_position = Some(PhysicalPosition::new(300.0, 150.0));

    app.begin_pointer_gesture(MouseButton::Right);
    app.end_pointer_gesture(MouseButton::Right);

    assert_eq!(app.interaction_mode, WorkbenchInteractionMode::Pan);
    assert_eq!(app.active_pointer_gesture, None);
    assert_eq!(app.interaction_override, None);
}

#[test]
fn outside_plot_press_starts_no_gesture() {
    let mut app = scatter_app_with_plot();
    app.cursor_position = Some(PhysicalPosition::new(99.0, 150.0));

    app.begin_pointer_gesture(MouseButton::Left);

    assert_eq!(app.active_pointer_gesture, None);
    assert_eq!(app.last_drag_position, None);
}

#[test]
fn consumed_release_still_ends_active_gesture() {
    let mut app = scatter_app_with_plot();
    app.cursor_position = Some(PhysicalPosition::new(300.0, 150.0));
    app.begin_pointer_gesture(MouseButton::Left);

    // Release routing calls this even when egui consumed the window event.
    app.end_pointer_gesture(MouseButton::Left);

    assert_eq!(app.active_pointer_gesture, None);
    assert_eq!(app.last_drag_position, None);
}

#[test]
fn escape_cancels_drag_before_clearing_selection() {
    let mut app = scatter_app_with_plot();
    app.scatter.active_brush_selection = Some(rawscope_render::ScatterBrushSelection {
        x_range: F32Range::new(10.0, 20.0),
        y_range: F32Range::new(30.0, 40.0),
    });
    app.cursor_position = Some(PhysicalPosition::new(300.0, 150.0));
    app.begin_pointer_gesture(MouseButton::Right);

    app.handle_escape();

    assert_eq!(app.active_pointer_gesture, None);
    assert!(app.scatter.active_brush_selection.is_some());

    app.handle_escape();

    assert!(app.scatter.active_brush_selection.is_none());
}

#[test]
fn inspect_hover_projects_plot_fraction_to_data_coordinates() {
    let mut app = scatter_app_with_plot();
    app.interaction_mode = WorkbenchInteractionMode::Inspect;

    app.update_pointer_position(PhysicalPosition::new(200.0, 100.0), true);

    assert_eq!(
        app.inspect_cursor_position,
        Some(InspectCursorPosition { x: 25.0, y: 150.0 })
    );
}

#[test]
fn mode_shortcuts_map_h_b_and_i() {
    assert_eq!(
        interaction_mode_for_shortcut(KeyCode::KeyH),
        Some(WorkbenchInteractionMode::Pan)
    );
    assert_eq!(
        interaction_mode_for_shortcut(KeyCode::KeyB),
        Some(WorkbenchInteractionMode::Brush)
    );
    assert_eq!(
        interaction_mode_for_shortcut(KeyCode::KeyI),
        Some(WorkbenchInteractionMode::Inspect)
    );
}

#[test]
fn cursor_feedback_tracks_mode_and_active_pan() {
    assert_eq!(
        cursor_icon(WorkbenchInteractionMode::Pan, None, true),
        CursorIcon::Grab
    );
    assert_eq!(
        cursor_icon(
            WorkbenchInteractionMode::Pan,
            Some(ActivePointerGesture::Pan),
            false,
        ),
        CursorIcon::Grabbing
    );
    assert_eq!(
        cursor_icon(WorkbenchInteractionMode::Inspect, None, true),
        CursorIcon::Crosshair
    );
}
