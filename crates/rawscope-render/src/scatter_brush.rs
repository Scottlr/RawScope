//! Scatter-density brush geometry and CPU-side selected-region summaries.

use rawscope_core::F32Range;
use rawscope_data::{SyntheticPointCategory, SyntheticPointRecord};

use crate::ScatterViewport;

const MIN_BRUSH_SCREEN_SPAN_PX: f32 = 1.0;

/// A screen-space point in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushScreenPoint {
    pub x: f32,
    pub y: f32,
}

impl BrushScreenPoint {
    /// Creates a screen-space brush point.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A screen-space size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushScreenSize {
    pub width: f32,
    pub height: f32,
}

impl BrushScreenSize {
    /// Creates a screen-space size.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    fn has_area(self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }
}

/// A normalized screen-space rectangle clamped to the active view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushScreenRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl BrushScreenRect {
    /// Normalizes two screen points into a clamped rectangle.
    pub fn from_points(
        start: BrushScreenPoint,
        end: BrushScreenPoint,
        screen_size: BrushScreenSize,
    ) -> Option<Self> {
        if !screen_size.has_area() {
            return None;
        }

        let start_x = start.x.clamp(0.0, screen_size.width);
        let start_y = start.y.clamp(0.0, screen_size.height);
        let end_x = end.x.clamp(0.0, screen_size.width);
        let end_y = end.y.clamp(0.0, screen_size.height);
        let min_x = start_x.min(end_x);
        let min_y = start_y.min(end_y);
        let max_x = start_x.max(end_x);
        let max_y = start_y.max(end_y);
        let brush_width_px = max_x - min_x;
        let brush_height_px = max_y - min_y;
        let brush_has_area = brush_width_px >= MIN_BRUSH_SCREEN_SPAN_PX
            && brush_height_px >= MIN_BRUSH_SCREEN_SPAN_PX;
        if !brush_has_area {
            return None;
        }

        Some(Self {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }
}

/// In-progress screen-space scatter brush drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterBrushDrag {
    pub screen_rect: BrushScreenRect,
}

impl ScatterBrushDrag {
    /// Builds a drag rectangle directly from two screen points.
    pub fn from_screen_points(
        start: BrushScreenPoint,
        end: BrushScreenPoint,
        screen_size: BrushScreenSize,
    ) -> Option<Self> {
        let screen_rect = BrushScreenRect::from_points(start, end, screen_size)?;
        Some(Self { screen_rect })
    }

    /// Finalizes this drag into a data-space selection for the current viewport.
    pub fn finalize(
        self,
        screen_size: BrushScreenSize,
        viewport: ScatterViewport,
    ) -> Option<ScatterBrushSelection> {
        ScatterBrushSelection::from_screen_rect(self.screen_rect, screen_size, viewport)
    }
}

/// Finalized data-space scatter brush selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterBrushSelection {
    pub x_range: F32Range,
    pub y_range: F32Range,
}

impl ScatterBrushSelection {
    /// Converts a screen-space rectangle into data-space x/y ranges.
    pub fn from_screen_rect(
        screen_rect: BrushScreenRect,
        screen_size: BrushScreenSize,
        viewport: ScatterViewport,
    ) -> Option<Self> {
        if !screen_size.has_area() {
            return None;
        }

        let min_x_fraction = screen_rect.min_x / screen_size.width;
        let max_x_fraction = screen_rect.max_x / screen_size.width;
        let min_y_fraction = screen_rect.min_y / screen_size.height;
        let max_y_fraction = screen_rect.max_y / screen_size.height;
        let (x_min, y_max) = viewport.data_point_at_fraction(min_x_fraction, min_y_fraction);
        let (x_max, y_min) = viewport.data_point_at_fraction(max_x_fraction, max_y_fraction);

        Some(Self {
            x_range: F32Range::new(x_min, x_max),
            y_range: F32Range::new(y_min, y_max),
        })
    }

    /// Builds a finalized data-space selection directly from two screen points.
    pub fn from_screen_points(
        start: BrushScreenPoint,
        end: BrushScreenPoint,
        screen_size: BrushScreenSize,
        viewport: ScatterViewport,
    ) -> Option<Self> {
        let screen_rect = BrushScreenRect::from_points(start, end, screen_size)?;
        Self::from_screen_rect(screen_rect, screen_size, viewport)
    }

    /// Projects this data-space selection into the current viewport.
    ///
    /// Fully off-screen selections are hidden. Partially visible selections are clamped to the
    /// viewport edge so the visible overlay never claims pixels outside the current view.
    pub fn project_to_screen(
        self,
        viewport: ScatterViewport,
        screen_size: BrushScreenSize,
    ) -> Option<BrushScreenRect> {
        if !screen_size.has_area() {
            return None;
        }

        let visible_x_range = intersect_range(self.x_range, viewport.x_range())?;
        let visible_y_range = intersect_range(self.y_range, viewport.y_range())?;
        let min_x_fraction =
            (visible_x_range.min - viewport.x_range().min) / viewport.x_range().span();
        let max_x_fraction =
            (visible_x_range.max - viewport.x_range().min) / viewport.x_range().span();
        let min_y_fraction =
            (viewport.y_range().max - visible_y_range.max) / viewport.y_range().span();
        let max_y_fraction =
            (viewport.y_range().max - visible_y_range.min) / viewport.y_range().span();

        Some(BrushScreenRect {
            min_x: min_x_fraction.clamp(0.0, 1.0) * screen_size.width,
            min_y: min_y_fraction.clamp(0.0, 1.0) * screen_size.height,
            max_x: max_x_fraction.clamp(0.0, 1.0) * screen_size.width,
            max_y: max_y_fraction.clamp(0.0, 1.0) * screen_size.height,
        })
    }

    /// Returns true when a point lies inside this brush's data-space ranges.
    pub fn contains_point(self, point: &SyntheticPointRecord) -> bool {
        self.x_range.contains(point.x) && self.y_range.contains(point.y)
    }
}

fn intersect_range(selection: F32Range, viewport: F32Range) -> Option<F32Range> {
    let visible_min = selection.min.max(viewport.min);
    let visible_max = selection.max.min(viewport.max);
    let range_is_visible = visible_max > visible_min;
    range_is_visible.then(|| F32Range::new(visible_min, visible_max))
}

/// Counts selected rows by synthetic point category.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SelectedCategoryCounts {
    pub cluster: usize,
    pub background: usize,
    pub outlier: usize,
}

impl SelectedCategoryCounts {
    /// Returns the largest selected category, if any rows were selected.
    pub fn top_category(self) -> Option<SyntheticPointCategory> {
        let categories = [
            (SyntheticPointCategory::Cluster, self.cluster),
            (SyntheticPointCategory::Background, self.background),
            (SyntheticPointCategory::Outlier, self.outlier),
        ];

        categories
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .and_then(|(category, count)| (count > 0).then_some(category))
    }

    fn add(&mut self, category: SyntheticPointCategory) {
        match category {
            SyntheticPointCategory::Cluster => self.cluster += 1,
            SyntheticPointCategory::Background => self.background += 1,
            SyntheticPointCategory::Outlier => self.outlier += 1,
        }
    }
}

/// CPU-side summary for the current scatter brush selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedRegionSummary {
    pub selected_row_count: usize,
    pub total_row_count: usize,
    pub selected_percentage: f32,
    pub brush_x_range: F32Range,
    pub brush_y_range: F32Range,
    pub selected_x_range: Option<F32Range>,
    pub selected_y_range: Option<F32Range>,
    pub category_counts: SelectedCategoryCounts,
    pub top_category: Option<SyntheticPointCategory>,
}

impl SelectedRegionSummary {
    /// Summarizes synthetic point records inside the given brush.
    pub fn from_points(points: &[SyntheticPointRecord], brush: ScatterBrushSelection) -> Self {
        let mut selected_row_count = 0;
        let mut selected_min_x = f32::INFINITY;
        let mut selected_max_x = f32::NEG_INFINITY;
        let mut selected_min_y = f32::INFINITY;
        let mut selected_max_y = f32::NEG_INFINITY;
        let mut category_counts = SelectedCategoryCounts::default();

        for point in points {
            let point_is_selected = brush.contains_point(point);
            if !point_is_selected {
                continue;
            }

            selected_row_count += 1;
            selected_min_x = selected_min_x.min(point.x);
            selected_max_x = selected_max_x.max(point.x);
            selected_min_y = selected_min_y.min(point.y);
            selected_max_y = selected_max_y.max(point.y);
            category_counts.add(point.category);
        }

        let selected_percentage = if points.is_empty() {
            0.0
        } else {
            selected_row_count as f32 / points.len() as f32 * 100.0
        };
        let selected_x_range = selected_range(selected_row_count, selected_min_x, selected_max_x);
        let selected_y_range = selected_range(selected_row_count, selected_min_y, selected_max_y);
        let top_category = category_counts.top_category();

        Self {
            selected_row_count,
            total_row_count: points.len(),
            selected_percentage,
            brush_x_range: brush.x_range,
            brush_y_range: brush.y_range,
            selected_x_range,
            selected_y_range,
            category_counts,
            top_category,
        }
    }
}

fn selected_range(selected_row_count: usize, min: f32, max: f32) -> Option<F32Range> {
    if selected_row_count == 0 {
        return None;
    }

    Some(F32Range::from_bounds_expanded(min, max))
}
