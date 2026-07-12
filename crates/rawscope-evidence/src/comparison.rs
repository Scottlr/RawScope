//! Render-independent comparison summaries embedded in evidence artifacts.

/// Share counts and percentages for one comparison bucket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComparisonRatio {
    pub selected_count: usize,
    pub baseline_count: usize,
    pub selected_percentage: f32,
    pub baseline_percentage: f32,
    pub delta_percentage_points: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionComparison {
    pub selected_row_count: usize,
    pub baseline_row_count: usize,
    pub selected_percentage: f32,
    pub point_kind_ratios: ScatterKindComparison,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterKindComparison {
    pub cluster: ComparisonRatio,
    pub background: ComparisonRatio,
    pub outlier: ComparisonRatio,
    pub unclassified: ComparisonRatio,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSelectionComparison {
    pub selected_event_count: usize,
    pub baseline_event_count: usize,
    pub selected_percentage: f32,
    pub event_kind_ratios: TimelineKindComparison,
    pub lane_ratios: Vec<ComparisonRatio>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineKindComparison {
    pub background: ComparisonRatio,
    pub spike: ComparisonRatio,
    pub stale_lane: ComparisonRatio,
    pub high_value_band: ComparisonRatio,
    pub unclassified: ComparisonRatio,
}
