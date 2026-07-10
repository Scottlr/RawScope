//! Selected-vs-baseline comparison summaries for visual selections.

use rawscope_data::{
    FilterMask, ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
};

use crate::{
    MissingnessSelectionSummary, SelectedCategoryCounts, SelectedEventTypeCounts,
    SelectedRegionSummary, TimelineSelectionSummary,
};

/// Share counts and percentages for one comparison bucket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComparisonRatio {
    pub selected_count: usize,
    pub baseline_count: usize,
    pub selected_percentage: f32,
    pub baseline_percentage: f32,
    pub delta_percentage_points: f32,
}

pub fn scatter_selection_comparison_masked(
    points: &[ScatterPointRecord],
    mask: &FilterMask,
    selection: crate::ScatterBrushSelection,
) -> Result<ScatterSelectionComparison, crate::MaskAlignmentError> {
    let summary = crate::selected_region_summary_masked(points, mask, selection)?;
    Ok(scatter_selection_comparison(points, summary))
}

/// Comparison summary for one finalized scatter selection.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSelectionComparison {
    pub selected_row_count: usize,
    pub baseline_row_count: usize,
    pub selected_percentage: f32,
    pub point_kind_ratios: ScatterKindComparison,
}

/// Point-kind shares for scatter comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterKindComparison {
    pub cluster: ComparisonRatio,
    pub background: ComparisonRatio,
    pub outlier: ComparisonRatio,
    pub unclassified: ComparisonRatio,
}

/// Comparison summary for one finalized timeline selection.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSelectionComparison {
    pub selected_event_count: usize,
    pub baseline_event_count: usize,
    pub selected_percentage: f32,
    pub event_kind_ratios: TimelineKindComparison,
    pub lane_ratios: Vec<ComparisonRatio>,
}

/// Event-kind shares for timeline comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineKindComparison {
    pub background: ComparisonRatio,
    pub spike: ComparisonRatio,
    pub stale_lane: ComparisonRatio,
    pub high_value_band: ComparisonRatio,
    pub unclassified: ComparisonRatio,
}

/// Comparison summary for one finalized missingness selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissingnessSelectionComparison {
    pub selected_missing_count: u64,
    pub selected_total_count: u64,
    pub baseline_missing_count: u64,
    pub baseline_total_count: u64,
    pub selected_missing_ratio: f32,
    pub baseline_missing_ratio: f32,
    pub delta_percentage_points: f32,
}

/// Builds a deterministic scatter selection comparison against the active point slice.
pub fn scatter_selection_comparison(
    points: &[ScatterPointRecord],
    summary: SelectedRegionSummary,
) -> ScatterSelectionComparison {
    let baseline_row_count = points.len();
    let baseline_category_counts = scatter_category_counts(points);

    ScatterSelectionComparison {
        selected_row_count: summary.selected_row_count,
        baseline_row_count,
        selected_percentage: summary.selected_percentage,
        point_kind_ratios: ScatterKindComparison {
            cluster: comparison_ratio(
                summary.category_counts.cluster,
                summary.selected_row_count,
                baseline_category_counts.cluster,
                baseline_row_count,
            ),
            background: comparison_ratio(
                summary.category_counts.background,
                summary.selected_row_count,
                baseline_category_counts.background,
                baseline_row_count,
            ),
            outlier: comparison_ratio(
                summary.category_counts.outlier,
                summary.selected_row_count,
                baseline_category_counts.outlier,
                baseline_row_count,
            ),
            unclassified: comparison_ratio(
                summary.category_counts.unclassified,
                summary.selected_row_count,
                baseline_category_counts.unclassified,
                baseline_row_count,
            ),
        },
    }
}

/// Builds a deterministic timeline selection comparison against the active event slice.
pub fn timeline_selection_comparison(
    events: &[TimelineEventRecord],
    lane_count: u32,
    summary: &TimelineSelectionSummary,
) -> TimelineSelectionComparison {
    let baseline_event_count = events.len();
    let baseline_event_type_counts = timeline_event_type_counts(events);
    let baseline_lane_counts = timeline_lane_counts(events, lane_count);

    let lane_ratios = baseline_lane_counts
        .iter()
        .copied()
        .enumerate()
        .map(|(lane_index, baseline_count)| {
            let selected_count = summary.lane_counts.get(lane_index).copied().unwrap_or(0);
            comparison_ratio(
                selected_count,
                summary.selected_event_count,
                baseline_count,
                baseline_event_count,
            )
        })
        .collect::<Vec<_>>();

    TimelineSelectionComparison {
        selected_event_count: summary.selected_event_count,
        baseline_event_count,
        selected_percentage: summary.selected_percentage,
        event_kind_ratios: TimelineKindComparison {
            background: comparison_ratio(
                summary.event_type_counts.background,
                summary.selected_event_count,
                baseline_event_type_counts.background,
                baseline_event_count,
            ),
            spike: comparison_ratio(
                summary.event_type_counts.spike,
                summary.selected_event_count,
                baseline_event_type_counts.spike,
                baseline_event_count,
            ),
            stale_lane: comparison_ratio(
                summary.event_type_counts.stale_lane,
                summary.selected_event_count,
                baseline_event_type_counts.stale_lane,
                baseline_event_count,
            ),
            high_value_band: comparison_ratio(
                summary.event_type_counts.high_value_band,
                summary.selected_event_count,
                baseline_event_type_counts.high_value_band,
                baseline_event_count,
            ),
            unclassified: comparison_ratio(
                summary.event_type_counts.unclassified,
                summary.selected_event_count,
                baseline_event_type_counts.unclassified,
                baseline_event_count,
            ),
        },
        lane_ratios,
    }
}

/// Builds a deterministic missingness selection comparison against the active source rows.
pub fn missingness_selection_comparison(
    selected: &MissingnessSelectionSummary,
    baseline_missing_count: u64,
    baseline_total_count: u64,
) -> MissingnessSelectionComparison {
    let selected_missing_ratio = selected.selected_missing_ratio();
    let baseline_missing_ratio = ratio_u64(baseline_missing_count, baseline_total_count);

    MissingnessSelectionComparison {
        selected_missing_count: selected.selected_missing_count,
        selected_total_count: selected.selected_total_count,
        baseline_missing_count,
        baseline_total_count,
        selected_missing_ratio,
        baseline_missing_ratio,
        delta_percentage_points: (selected_missing_ratio - baseline_missing_ratio) * 100.0,
    }
}

fn comparison_ratio(
    selected_count: usize,
    selected_total_count: usize,
    baseline_count: usize,
    baseline_total_count: usize,
) -> ComparisonRatio {
    let selected_percentage = percentage_usize(selected_count, selected_total_count);
    let baseline_percentage = percentage_usize(baseline_count, baseline_total_count);

    ComparisonRatio {
        selected_count,
        baseline_count,
        selected_percentage,
        baseline_percentage,
        delta_percentage_points: selected_percentage - baseline_percentage,
    }
}

fn percentage_usize(count: usize, total_count: usize) -> f32 {
    if total_count == 0 {
        return 0.0;
    }

    count as f32 / total_count as f32 * 100.0
}

fn ratio_u64(count: u64, total_count: u64) -> f32 {
    if total_count == 0 {
        return 0.0;
    }

    count as f32 / total_count as f32
}

fn scatter_category_counts(points: &[ScatterPointRecord]) -> SelectedCategoryCounts {
    let mut category_counts = SelectedCategoryCounts::default();

    for point in points {
        match point.kind {
            ScatterPointKind::Synthetic(rawscope_data::SyntheticPointCategory::Cluster) => {
                category_counts.cluster += 1
            }
            ScatterPointKind::Synthetic(rawscope_data::SyntheticPointCategory::Background) => {
                category_counts.background += 1
            }
            ScatterPointKind::Synthetic(rawscope_data::SyntheticPointCategory::Outlier) => {
                category_counts.outlier += 1
            }
            ScatterPointKind::Unclassified => category_counts.unclassified += 1,
        }
    }

    category_counts
}

fn timeline_event_type_counts(events: &[TimelineEventRecord]) -> SelectedEventTypeCounts {
    let mut event_type_counts = SelectedEventTypeCounts::default();

    for event in events {
        match event.kind {
            TimelineEventKind::Synthetic(rawscope_data::SyntheticEventType::Background) => {
                event_type_counts.background += 1
            }
            TimelineEventKind::Synthetic(rawscope_data::SyntheticEventType::Spike) => {
                event_type_counts.spike += 1
            }
            TimelineEventKind::Synthetic(rawscope_data::SyntheticEventType::StaleLane) => {
                event_type_counts.stale_lane += 1
            }
            TimelineEventKind::Synthetic(rawscope_data::SyntheticEventType::HighValueBand) => {
                event_type_counts.high_value_band += 1
            }
            TimelineEventKind::Unclassified => event_type_counts.unclassified += 1,
        }
    }

    event_type_counts
}

fn timeline_lane_counts(events: &[TimelineEventRecord], lane_count: u32) -> Vec<usize> {
    let mut lane_counts = vec![0; lane_count as usize];

    for event in events {
        if let Some(lane_count) = lane_counts.get_mut(event.lane as usize) {
            *lane_count += 1;
        }
    }

    lane_counts
}
