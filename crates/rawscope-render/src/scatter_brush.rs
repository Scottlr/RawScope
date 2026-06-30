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

/// Data-space ranges represented by a scatter brush rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterBrush {
    pub x_range: F32Range,
    pub y_range: F32Range,
    pub screen_rect: BrushScreenRect,
}

impl ScatterBrush {
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
            screen_rect,
        })
    }

    /// Builds a brush directly from two screen points.
    pub fn from_screen_points(
        start: BrushScreenPoint,
        end: BrushScreenPoint,
        screen_size: BrushScreenSize,
        viewport: ScatterViewport,
    ) -> Option<Self> {
        let screen_rect = BrushScreenRect::from_points(start, end, screen_size)?;
        Self::from_screen_rect(screen_rect, screen_size, viewport)
    }

    /// Returns true when a point lies inside this brush's data-space ranges.
    pub fn contains_point(self, point: &SyntheticPointRecord) -> bool {
        self.x_range.contains(point.x) && self.y_range.contains(point.y)
    }
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
    pub fn from_points(points: &[SyntheticPointRecord], brush: ScatterBrush) -> Self {
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

    if max > min {
        Some(F32Range::new(min, max))
    } else {
        let epsilon = f32::EPSILON.max(min.abs() * f32::EPSILON);
        Some(F32Range::new(min - epsilon, max + epsilon))
    }
}

#[cfg(test)]
mod tests {
    use rawscope_core::RowId;

    use super::*;

    fn viewport() -> ScatterViewport {
        ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 100.0))
    }

    fn point(
        row_id: u64,
        x: f32,
        y: f32,
        category: SyntheticPointCategory,
    ) -> SyntheticPointRecord {
        SyntheticPointRecord {
            row_id: RowId(row_id),
            x,
            y,
            category,
        }
    }

    #[test]
    fn screen_rectangle_normalizes_drag_direction() {
        let rect = BrushScreenRect::from_points(
            BrushScreenPoint::new(80.0, 70.0),
            BrushScreenPoint::new(10.0, 20.0),
            BrushScreenSize::new(100.0, 100.0),
        )
        .unwrap();

        assert_eq!(rect.min_x, 10.0);
        assert_eq!(rect.min_y, 20.0);
        assert_eq!(rect.max_x, 80.0);
        assert_eq!(rect.max_y, 70.0);
    }

    #[test]
    fn screen_rectangle_clamps_outside_viewport() {
        let rect = BrushScreenRect::from_points(
            BrushScreenPoint::new(-10.0, -20.0),
            BrushScreenPoint::new(120.0, 130.0),
            BrushScreenSize::new(100.0, 100.0),
        )
        .unwrap();

        assert_eq!(rect.min_x, 0.0);
        assert_eq!(rect.min_y, 0.0);
        assert_eq!(rect.max_x, 100.0);
        assert_eq!(rect.max_y, 100.0);
    }

    #[test]
    fn brush_converts_screen_rect_to_data_ranges() {
        let brush = ScatterBrush::from_screen_points(
            BrushScreenPoint::new(25.0, 25.0),
            BrushScreenPoint::new(75.0, 75.0),
            BrushScreenSize::new(100.0, 100.0),
            viewport(),
        )
        .unwrap();

        assert_eq!(brush.x_range, F32Range::new(25.0, 75.0));
        assert_eq!(brush.y_range, F32Range::new(25.0, 75.0));
    }

    #[test]
    fn brush_clear_reset_is_represented_by_absent_selection() {
        let mut brush = Some(
            ScatterBrush::from_screen_points(
                BrushScreenPoint::new(10.0, 10.0),
                BrushScreenPoint::new(20.0, 20.0),
                BrushScreenSize::new(100.0, 100.0),
                viewport(),
            )
            .unwrap(),
        );
        assert!(brush.is_some());

        brush = None;

        assert_eq!(brush, None);
    }

    #[test]
    fn selected_summary_counts_rows_in_brush() {
        let brush = ScatterBrush::from_screen_points(
            BrushScreenPoint::new(0.0, 50.0),
            BrushScreenPoint::new(50.0, 100.0),
            BrushScreenSize::new(100.0, 100.0),
            viewport(),
        )
        .unwrap();
        let points = vec![
            point(0, 10.0, 10.0, SyntheticPointCategory::Cluster),
            point(1, 40.0, 40.0, SyntheticPointCategory::Background),
            point(2, 90.0, 90.0, SyntheticPointCategory::Outlier),
        ];

        let summary = SelectedRegionSummary::from_points(&points, brush);

        assert_eq!(summary.selected_row_count, 2);
        assert_eq!(summary.total_row_count, 3);
        assert!((summary.selected_percentage - 66.66667).abs() < 0.001);
    }

    #[test]
    fn selected_summary_counts_categories() {
        let brush = ScatterBrush::from_screen_points(
            BrushScreenPoint::new(0.0, 0.0),
            BrushScreenPoint::new(100.0, 100.0),
            BrushScreenSize::new(100.0, 100.0),
            viewport(),
        )
        .unwrap();
        let points = vec![
            point(0, 10.0, 10.0, SyntheticPointCategory::Cluster),
            point(1, 20.0, 20.0, SyntheticPointCategory::Cluster),
            point(2, 30.0, 30.0, SyntheticPointCategory::Outlier),
        ];

        let summary = SelectedRegionSummary::from_points(&points, brush);

        assert_eq!(summary.category_counts.cluster, 2);
        assert_eq!(summary.category_counts.background, 0);
        assert_eq!(summary.category_counts.outlier, 1);
        assert_eq!(summary.top_category, Some(SyntheticPointCategory::Cluster));
    }
}
