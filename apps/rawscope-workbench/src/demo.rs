//! Workbench density-view controls and title helpers.

use rawscope_render::{
    ScatterDensityRenderStats, ScatterSelectionEvidence, ScatterViewport, SelectedRegionSummary,
    SelectionDrilldown, TimelineDensityRenderStats, TimelineSelectionEvidence,
    TimelineSelectionSummary, TimelineViewport,
};

/// Workbench demo selected at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DemoMode {
    #[default]
    Scatter,
    Timeline,
}

impl DemoMode {
    pub(crate) fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "scatter" => Ok(Self::Scatter),
            "timeline" => Ok(Self::Timeline),
            _ => Err(format!(
                "unsupported demo '{name}'; use scatter or timeline"
            )),
        }
    }

    /// Returns true for the scatter-density demo mode.
    pub fn is_scatter(self) -> bool {
        self == Self::Scatter
    }

    /// Returns true for the timeline-density demo mode.
    pub fn is_timeline(self) -> bool {
        self == Self::Timeline
    }
}

/// Keyboard-selectable deterministic synthetic point-count preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointCountPreset {
    pub key_label: &'static str,
    pub row_count: usize,
}

impl PointCountPreset {
    /// Returns the default demo preset.
    pub fn default() -> Self {
        POINT_COUNT_PRESETS[0]
    }

    /// Returns the preset selected by a number key.
    pub fn from_digit_key(digit: char) -> Option<Self> {
        POINT_COUNT_PRESETS
            .iter()
            .copied()
            .find(|preset| preset.key_label == digit.to_string())
    }

    /// Returns a compact display label for the row count.
    pub fn row_count_label(self) -> &'static str {
        match self.row_count {
            20_000 => "20k",
            200_000 => "200k",
            1_000_000 => "1M",
            5_000_000 => "5M",
            _ => "custom",
        }
    }
}

impl Default for PointCountPreset {
    fn default() -> Self {
        POINT_COUNT_PRESETS[0]
    }
}

/// Presets used by the current synthetic scatter-density demo.
pub const POINT_COUNT_PRESETS: [PointCountPreset; 4] = [
    PointCountPreset {
        key_label: "1",
        row_count: 20_000,
    },
    PointCountPreset {
        key_label: "2",
        row_count: 200_000,
    },
    PointCountPreset {
        key_label: "3",
        row_count: 1_000_000,
    },
    PointCountPreset {
        key_label: "4",
        row_count: 5_000_000,
    },
];

/// State displayed in the scatter window title.
#[derive(Debug, Clone, PartialEq)]
pub struct DemoOverlayState {
    pub point_count_label: String,
    pub viewport: ScatterViewport,
    pub render_stats: ScatterDensityRenderStats,
    pub selection_summary: Option<SelectedRegionSummary>,
    pub selection_evidence: Option<ScatterSelectionEvidence>,
    pub selection_drilldown: Option<SelectionDrilldown>,
}

impl DemoOverlayState {
    /// Formats compact state suitable for a winit window title.
    pub fn title(&self) -> String {
        let selection_summary = self
            .selection_evidence
            .as_ref()
            .map(format_evidence_summary)
            .or_else(|| self.selection_summary.map(format_selection_summary))
            .unwrap_or_else(|| "selection none".to_string());
        let drilldown_summary = self
            .selection_drilldown
            .as_ref()
            .map(format_drilldown_summary)
            .unwrap_or_default();

        format!(
            "RawScope | pts {} ({}) | grid {}x{} | x {:.1}..{:.1} y {:.1}..{:.1} | max {} | {}{} | wheel zoom, drag pan, right/shift-drag brush, Esc clear, R reset, 1-4 presets",
            self.render_stats.point_count,
            self.point_count_label,
            self.render_stats.grid_width,
            self.render_stats.grid_height,
            self.viewport.x_range().min,
            self.viewport.x_range().max,
            self.viewport.y_range().min,
            self.viewport.y_range().max,
            self.render_stats.max_bin_count,
            selection_summary,
            drilldown_summary,
        )
    }
}

/// State displayed in the timeline window title.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineOverlayState {
    pub viewport: TimelineViewport,
    pub render_stats: TimelineDensityRenderStats,
    pub selection_summary: Option<TimelineSelectionSummary>,
    pub selection_evidence: Option<TimelineSelectionEvidence>,
    pub selection_drilldown: Option<SelectionDrilldown>,
}

impl TimelineOverlayState {
    /// Formats compact timeline state suitable for a winit window title.
    pub fn title(&self) -> String {
        let selection_summary = self
            .selection_evidence
            .as_ref()
            .map(format_timeline_evidence_summary)
            .or_else(|| {
                self.selection_summary
                    .as_ref()
                    .map(format_timeline_selection_summary)
            })
            .unwrap_or_else(|| "selection none".to_string());
        let drilldown_summary = self
            .selection_drilldown
            .as_ref()
            .map(format_drilldown_summary)
            .unwrap_or_default();

        format!(
            "RawScope | timeline events {} | lanes {} | grid {}x{} | time {}..{} full {}..{} | max {} | {}{} | wheel zoom time, drag pan time, right/shift-drag brush, Esc clear, R reset, --demo scatter for scatter view",
            self.render_stats.event_count,
            self.render_stats.lane_count,
            self.render_stats.grid_width,
            self.render_stats.grid_height,
            self.viewport.time_range().min,
            self.viewport.time_range().max,
            self.viewport.full_time_range().min,
            self.viewport.full_time_range().max,
            self.render_stats.max_bin_count,
            selection_summary,
            drilldown_summary,
        )
    }
}

fn format_drilldown_summary(drilldown: &SelectionDrilldown) -> String {
    format!(
        " drilldown {}/{}{}",
        drilldown.displayed_row_count,
        drilldown.selected_row_count,
        if drilldown.rows_are_sampled {
            " sampled"
        } else {
            ""
        }
    )
}

fn format_timeline_evidence_summary(evidence: &TimelineSelectionEvidence) -> String {
    let row_id_sample = evidence
        .selected_row_id_sample
        .iter()
        .take(5)
        .map(|row_id| row_id.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selected_timestamp_range = evidence
        .selected_timestamp_range
        .map(|range| format!("{}..{}", range.min, range.max))
        .unwrap_or_else(|| "none".to_string());
    let selected_value_range = evidence
        .selected_value_range
        .map(|(min, max)| format!("{min:.1}..{max:.1}"))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "evidence events {} ({:.2}%) sample [{}] brush t {}..{} lanes {}..{} data t {} value {} top lane {:?} top type {:?}",
        evidence.selected_event_count,
        evidence.selected_percentage,
        row_id_sample,
        evidence.selected_time_range.min,
        evidence.selected_time_range.max,
        evidence.selected_lane_range.start,
        evidence.selected_lane_range.end_exclusive,
        selected_timestamp_range,
        selected_value_range,
        evidence.top_lane,
        evidence.top_event_type,
    )
}

fn format_timeline_selection_summary(summary: &TimelineSelectionSummary) -> String {
    let selected_timestamp_range = summary
        .selected_timestamp_range
        .map(|range| format!("{}..{}", range.min, range.max))
        .unwrap_or_else(|| "none".to_string());
    let selected_value_range = summary
        .selected_value_range
        .map(|(min, max)| format!("{min:.1}..{max:.1}"))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "sel events {} ({:.2}%) brush t {}..{} lanes {}..{} data t {} value {} top lane {:?} top type {:?}",
        summary.selected_event_count,
        summary.selected_percentage,
        summary.selected_time_range.min,
        summary.selected_time_range.max,
        summary.selected_lane_range.start,
        summary.selected_lane_range.end_exclusive,
        selected_timestamp_range,
        selected_value_range,
        summary.top_lane,
        summary.top_event_type,
    )
}

fn format_evidence_summary(evidence: &ScatterSelectionEvidence) -> String {
    let row_id_sample = evidence
        .selected_row_id_sample
        .iter()
        .take(5)
        .map(|row_id| row_id.0.to_string())
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "evidence rows {} ({:.2}%) sample [{}] cats c:{} b:{} o:{} u:{} top {:?}",
        evidence.selected_row_count,
        evidence.selected_percentage,
        row_id_sample,
        evidence.category_counts.cluster,
        evidence.category_counts.background,
        evidence.category_counts.outlier,
        evidence.category_counts.unclassified,
        evidence.top_category,
    )
}

fn format_selection_summary(summary: SelectedRegionSummary) -> String {
    let selected_x_range = summary
        .selected_x_range
        .map(|range| format!("{:.1}..{:.1}", range.min, range.max))
        .unwrap_or_else(|| "none".to_string());
    let selected_y_range = summary
        .selected_y_range
        .map(|range| format!("{:.1}..{:.1}", range.min, range.max))
        .unwrap_or_else(|| "none".to_string());
    let top_category = summary
        .top_category
        .map(|category| format!("{category:?}"))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "sel {} ({:.2}%) brush x {:.1}..{:.1} y {:.1}..{:.1} data x {} y {} cats c:{} b:{} o:{} u:{} top {}",
        summary.selected_row_count,
        summary.selected_percentage,
        summary.brush_x_range.min,
        summary.brush_x_range.max,
        summary.brush_y_range.min,
        summary.brush_y_range.max,
        selected_x_range,
        selected_y_range,
        summary.category_counts.cluster,
        summary.category_counts.background,
        summary.category_counts.outlier,
        summary.category_counts.unclassified,
        top_category,
    )
}

#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, U64Range};
    use rawscope_render::{
        ScatterDensityRenderStats, ScatterViewport, SelectionDrilldown, TimelineDensityRenderStats,
        TimelineViewport,
    };

    use super::*;

    #[test]
    fn digit_keys_select_expected_point_presets() {
        assert_eq!(
            PointCountPreset::from_digit_key('1').unwrap().row_count,
            20_000
        );
        assert_eq!(
            PointCountPreset::from_digit_key('2').unwrap().row_count,
            200_000
        );
        assert_eq!(
            PointCountPreset::from_digit_key('3').unwrap().row_count,
            1_000_000
        );
        assert_eq!(
            PointCountPreset::from_digit_key('4').unwrap().row_count,
            5_000_000
        );
    }

    #[test]
    fn unsupported_digit_key_selects_no_preset() {
        assert_eq!(PointCountPreset::from_digit_key('5'), None);
    }

    #[test]
    fn title_includes_core_overlay_fields() {
        let viewport = ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 100.0));
        let overlay = DemoOverlayState {
            point_count_label: PointCountPreset::default().row_count_label().to_string(),
            viewport,
            render_stats: ScatterDensityRenderStats {
                point_count: 20_000,
                grid_width: 256,
                grid_height: 256,
                max_bin_count: 42,
            },
            selection_summary: None,
            selection_evidence: None,
            selection_drilldown: None,
        };

        let title = overlay.title();

        assert!(title.contains("pts 20000"));
        assert!(title.contains("grid 256x256"));
        assert!(title.contains("max 42"));
        assert!(title.contains("wheel zoom"));
        assert!(title.contains("selection none"));
    }

    #[test]
    fn timeline_title_includes_core_overlay_fields() {
        let viewport = TimelineViewport::new(U64Range::new(0, 1_000), 8);
        let overlay = TimelineOverlayState {
            viewport,
            render_stats: TimelineDensityRenderStats {
                event_count: 20_000,
                lane_count: 8,
                grid_width: 256,
                grid_height: 8,
                max_bin_count: 99,
            },
            selection_summary: None,
            selection_evidence: None,
            selection_drilldown: None,
        };

        let title = overlay.title();

        assert!(title.contains("timeline events 20000"));
        assert!(title.contains("lanes 8"));
        assert!(title.contains("grid 256x8"));
        assert!(title.contains("max 99"));
        assert!(title.contains("wheel zoom time"));
        assert!(title.contains("--demo scatter"));
    }

    #[test]
    fn title_includes_drilldown_hint_when_present() {
        let viewport = ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 100.0));
        let overlay = DemoOverlayState {
            point_count_label: PointCountPreset::default().row_count_label().to_string(),
            viewport,
            render_stats: ScatterDensityRenderStats {
                point_count: 20_000,
                grid_width: 256,
                grid_height: 256,
                max_bin_count: 42,
            },
            selection_summary: None,
            selection_evidence: None,
            selection_drilldown: Some(SelectionDrilldown {
                selected_row_count: 24,
                displayed_row_count: 10,
                rows_are_sampled: true,
                columns: vec![],
                rows: vec![],
            }),
        };

        let title = overlay.title();

        assert!(title.contains("drilldown 10/24 sampled"));
    }
}
