//! Checked physical plot geometry shared by rendering and interaction.

use std::fmt;

use crate::scatter_brush::{BrushScreenPoint, BrushScreenSize};

/// A non-empty plot rectangle in physical surface pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlotRectPx {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A point in physical surface pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotPointPx {
    pub x: f32,
    pub y: f32,
}

impl PlotPointPx {
    /// Creates a physical surface point.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Validation failures for a physical plot rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotGeometryError {
    ZeroWidth,
    ZeroHeight,
    OutsideSurface,
}

impl fmt::Display for PlotGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroWidth => "plot width must be non-zero",
            Self::ZeroHeight => "plot height must be non-zero",
            Self::OutsideSurface => "plot rectangle must fit inside the surface",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PlotGeometryError {}

impl PlotRectPx {
    /// Creates a checked plot rectangle bounded by the physical surface.
    pub fn try_new(
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        surface_width: u32,
        surface_height: u32,
    ) -> Result<Self, PlotGeometryError> {
        if width == 0 {
            return Err(PlotGeometryError::ZeroWidth);
        }
        if height == 0 {
            return Err(PlotGeometryError::ZeroHeight);
        }

        let right = x
            .checked_add(width)
            .ok_or(PlotGeometryError::OutsideSurface)?;
        let bottom = y
            .checked_add(height)
            .ok_or(PlotGeometryError::OutsideSurface)?;
        if right > surface_width || bottom > surface_height {
            return Err(PlotGeometryError::OutsideSurface);
        }

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Returns true when a finite point lies on or inside the plot edges.
    pub fn contains(self, point: PlotPointPx) -> bool {
        if !point.x.is_finite() || !point.y.is_finite() {
            return false;
        }

        let right = self.x as f32 + self.width as f32;
        let bottom = self.y as f32 + self.height as f32;
        point.x >= self.x as f32
            && point.x <= right
            && point.y >= self.y as f32
            && point.y <= bottom
    }

    /// Maps an in-plot physical point to plot-local fractions.
    pub fn fraction_at(self, point: PlotPointPx) -> Option<(f32, f32)> {
        if !self.contains(point) {
            return None;
        }

        let x_fraction = (point.x - self.x as f32) / self.width as f32;
        let y_fraction = (point.y - self.y as f32) / self.height as f32;
        Some((x_fraction.clamp(0.0, 1.0), y_fraction.clamp(0.0, 1.0)))
    }

    /// Converts an in-plot physical point to plot-local brush pixels.
    pub fn local_point(self, point: PlotPointPx) -> Option<BrushScreenPoint> {
        self.contains(point)
            .then(|| BrushScreenPoint::new(point.x - self.x as f32, point.y - self.y as f32))
    }

    /// Converts a finite physical point to plot-local pixels, clamped to the plot.
    ///
    /// This is used only after a gesture has started inside the plot so drags can
    /// continue and finalize when the pointer leaves the plot bounds.
    pub fn clamped_local_point(self, point: PlotPointPx) -> Option<BrushScreenPoint> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }

        let local_x = (point.x - self.x as f32).clamp(0.0, self.width as f32);
        let local_y = (point.y - self.y as f32).clamp(0.0, self.height as f32);
        Some(BrushScreenPoint::new(local_x, local_y))
    }

    /// Returns the plot-local brush extent.
    pub fn screen_size(self) -> BrushScreenSize {
        BrushScreenSize::new(self.width as f32, self.height as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_rect_maps_edges_and_center_to_fractions() {
        let plot = PlotRectPx::try_new(100, 50, 800, 400, 1_000, 600).unwrap();

        assert_eq!(
            plot.fraction_at(PlotPointPx::new(100.0, 50.0)),
            Some((0.0, 0.0))
        );
        assert_eq!(
            plot.fraction_at(PlotPointPx::new(500.0, 250.0)),
            Some((0.5, 0.5))
        );
        assert_eq!(
            plot.fraction_at(PlotPointPx::new(900.0, 450.0)),
            Some((1.0, 1.0))
        );
        assert_eq!(plot.fraction_at(PlotPointPx::new(99.0, 250.0)), None);
    }

    #[test]
    fn plot_rect_rejects_empty_or_out_of_surface_bounds() {
        assert_eq!(
            PlotRectPx::try_new(0, 0, 0, 10, 100, 100),
            Err(PlotGeometryError::ZeroWidth)
        );
        assert_eq!(
            PlotRectPx::try_new(0, 0, 10, 0, 100, 100),
            Err(PlotGeometryError::ZeroHeight)
        );
        assert_eq!(
            PlotRectPx::try_new(90, 0, 20, 10, 100, 100),
            Err(PlotGeometryError::OutsideSurface)
        );
        assert_eq!(
            PlotRectPx::try_new(u32::MAX, 0, 2, 10, u32::MAX, 100),
            Err(PlotGeometryError::OutsideSurface)
        );
    }
}
