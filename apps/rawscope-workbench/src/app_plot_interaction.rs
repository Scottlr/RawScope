//! Plot-local scatter viewport interaction coordination.

use rawscope_render::{BrushScreenPoint, BrushScreenSize, PlotPointPx};
use tracing::error;
use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

use crate::app::{WorkbenchApp, WHEEL_ZOOM_IN_SCALE, WHEEL_ZOOM_OUT_SCALE};

impl WorkbenchApp {
    /// Returns the current cursor as plot-local fractions when it is inside the plot.
    pub(crate) fn cursor_fraction(&self) -> Option<(f32, f32)> {
        let plot_rect = self.plot_surface?.physical_rect;
        let cursor_position = self.cursor_position?;
        plot_rect.fraction_at(plot_point(cursor_position))
    }

    /// Returns the current cursor in plot-local physical pixels when it is inside.
    pub(crate) fn plot_local_cursor_point(&self) -> Option<BrushScreenPoint> {
        let cursor_position = self.cursor_position?;
        self.plot_local_point(cursor_position)
    }

    /// Converts an in-plot physical cursor position to plot-local pixels.
    pub(crate) fn plot_local_point(
        &self,
        position: PhysicalPosition<f64>,
    ) -> Option<BrushScreenPoint> {
        self.plot_surface?
            .physical_rect
            .local_point(plot_point(position))
    }

    /// Converts an active gesture position to plot-local pixels clamped to the plot.
    pub(crate) fn clamped_plot_local_point(
        &self,
        position: PhysicalPosition<f64>,
    ) -> Option<BrushScreenPoint> {
        self.plot_surface?
            .physical_rect
            .clamped_local_point(plot_point(position))
    }

    /// Returns the current plot-local physical extent.
    pub(crate) fn plot_screen_size(&self) -> Option<BrushScreenSize> {
        Some(self.plot_surface?.physical_rect.screen_size())
    }

    pub(crate) fn zoom_at_cursor(&mut self, scroll_delta: MouseScrollDelta) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        let Some(cursor_fraction) = self.cursor_fraction() else {
            return;
        };
        let Some(viewport) = self.scatter.viewport.as_mut() else {
            return;
        };

        let zoom_scroll = match scroll_delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => position.y as f32,
        };
        if zoom_scroll == 0.0 {
            return;
        }

        let zoom_scale = if zoom_scroll > 0.0 {
            WHEEL_ZOOM_IN_SCALE
        } else {
            WHEEL_ZOOM_OUT_SCALE
        };
        let (anchor_x, anchor_y) =
            viewport.data_point_at_fraction(cursor_fraction.0, cursor_fraction.1);

        viewport.zoom_around(anchor_x, anchor_y, zoom_scale);
        self.begin_interactive_density();
        self.interactive_viewport_changed();
        self.finish_interactive_density();
    }

    pub(crate) fn begin_pan(&mut self) {
        if !self.demo_mode.is_scatter() || self.cursor_fraction().is_none() {
            return;
        }

        self.last_drag_position = self.cursor_position;
        self.begin_interactive_density();
    }

    pub(crate) fn end_pan(&mut self) {
        if self.last_drag_position.is_some() && self.demo_mode.is_scatter() {
            self.finish_interactive_density();
        }
        self.last_drag_position = None;
    }

    pub(crate) fn pan_to_cursor(&mut self, position: PhysicalPosition<f64>) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        let Some(last_drag_position) = self.last_drag_position else {
            return;
        };
        let Some(plot_size) = self.plot_screen_size() else {
            return;
        };
        let Some(viewport) = self.scatter.viewport.as_mut() else {
            return;
        };

        let delta_x_fraction = (position.x - last_drag_position.x) as f32 / plot_size.width;
        let delta_y_fraction = (position.y - last_drag_position.y) as f32 / plot_size.height;
        let data_delta_x = -delta_x_fraction * viewport.x_range().span();
        let data_delta_y = -delta_y_fraction * viewport.y_range().span();

        viewport.pan_by(data_delta_x, data_delta_y);
        self.last_drag_position = Some(position);
        self.interactive_viewport_changed();
    }

    pub(crate) fn reset_viewport(&mut self) {
        if !self.demo_mode.is_scatter() {
            return;
        }

        let Some(viewport) = self.scatter.viewport.as_mut() else {
            return;
        };

        viewport.reset();
        if let Err(err) = self.recompute_density() {
            error!(error = %err, "failed to recompute scatter density after reset");
        }
    }
}

fn plot_point(position: PhysicalPosition<f64>) -> PlotPointPx {
    PlotPointPx::new(position.x as f32, position.y as f32)
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};
    use rawscope_core::F32Range;
    use rawscope_render::{PlotRectPx, ScatterViewport};

    use super::*;
    use crate::{
        demo::DemoMode,
        ui_plot_surface::{PlotAxisLayout, PlotSurfaceLayout},
    };

    fn app_with_plot() -> WorkbenchApp {
        let plot_rect = PlotRectPx::try_new(100, 50, 400, 200, 800, 600).unwrap();
        let mut viewport =
            ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 100.0));
        viewport.zoom_around(50.0, 50.0, 0.5);

        WorkbenchApp {
            demo_mode: DemoMode::Scatter,
            plot_surface: Some(PlotSurfaceLayout {
                logical_rect: Rect::from_min_max(pos2(50.0, 25.0), pos2(250.0, 125.0)),
                physical_rect: plot_rect,
                axis_layout: PlotAxisLayout {
                    outer_rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(262.0, 163.0)),
                    plot_rect: Rect::from_min_max(pos2(50.0, 25.0), pos2(250.0, 125.0)),
                },
            }),
            scatter: crate::app::ScatterWorkbenchState {
                viewport: Some(viewport),
                ..crate::app::ScatterWorkbenchState::default()
            },
            ..WorkbenchApp::default()
        }
    }

    #[test]
    fn cursor_fraction_is_plot_local_and_rejects_outside_starts() {
        let mut app = app_with_plot();
        app.cursor_position = Some(PhysicalPosition::new(300.0, 150.0));

        assert_eq!(app.cursor_fraction(), Some((0.5, 0.5)));
        app.begin_pan();
        assert_eq!(app.last_drag_position, app.cursor_position);

        app.end_pan();
        app.cursor_position = Some(PhysicalPosition::new(99.0, 150.0));
        assert_eq!(app.cursor_fraction(), None);
        app.begin_pan();
        assert_eq!(app.last_drag_position, None);
    }

    #[test]
    fn scatter_pan_uses_plot_dimensions_and_preserves_approved_signs() {
        let mut app = app_with_plot();
        app.last_drag_position = Some(PhysicalPosition::new(300.0, 150.0));

        app.pan_to_cursor(PhysicalPosition::new(340.0, 170.0));

        let viewport = app.scatter.viewport.unwrap();
        assert_eq!(viewport.x_range(), F32Range::new(20.0, 70.0));
        assert_eq!(viewport.y_range(), F32Range::new(20.0, 70.0));
    }

    #[test]
    fn pan_directions_remain_consistent_during_reprojection() {
        let mut app = app_with_plot();
        app.cursor_position = Some(PhysicalPosition::new(300.0, 150.0));
        app.begin_pan();

        app.pan_to_cursor(PhysicalPosition::new(340.0, 170.0));

        let viewport = app.scatter.viewport.unwrap();
        assert_eq!(viewport.x_range(), F32Range::new(20.0, 70.0));
        assert_eq!(viewport.y_range(), F32Range::new(20.0, 70.0));
        assert!(app.render_schedule.is_refining());
        assert!(app.scatter.marginal_summary.is_none());
    }
}
