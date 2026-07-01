//! Workbench scatter-density demo controls and diagnostics helpers.

use std::time::Duration;

use rawscope_render::{
    ScatterDensityRenderDiagnostics, ScatterSelectionEvidence, ScatterViewport,
    SelectedRegionSummary, TimelineDensityRenderDiagnostics, TimelineViewport,
};

/// Workbench demo selected at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DemoMode {
    #[default]
    Scatter,
    Timeline,
}

impl DemoMode {
    /// Parses the optional workbench demo command-line argument.
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let Some(first_arg) = args.next() else {
            return Ok(Self::Scatter);
        };

        if let Some(demo_name) = first_arg.strip_prefix("--demo=") {
            return Self::from_name(demo_name);
        }

        if first_arg == "--demo" {
            let Some(demo_name) = args.next() else {
                return Err("missing demo name after --demo; use scatter or timeline".to_string());
            };
            let has_extra_args = args.next().is_some();
            if has_extra_args {
                return Err("unexpected extra arguments after --demo".to_string());
            }
            return Self::from_name(&demo_name);
        }

        Err(format!(
            "unsupported argument '{first_arg}'; use --demo scatter or --demo timeline"
        ))
    }

    fn from_name(name: &str) -> Result<Self, String> {
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

/// State displayed in the window-title diagnostics overlay.
#[derive(Debug, Clone, PartialEq)]
pub struct DemoOverlayState {
    pub preset: PointCountPreset,
    pub viewport: ScatterViewport,
    pub render_diagnostics: ScatterDensityRenderDiagnostics,
    pub selection_summary: Option<SelectedRegionSummary>,
    pub selection_evidence: Option<ScatterSelectionEvidence>,
    pub redraw_count: u64,
    pub latest_frame_cpu_duration: Duration,
    pub adapter_name: String,
    pub backend: String,
}

impl DemoOverlayState {
    /// Formats compact diagnostics suitable for a winit window title.
    pub fn title(&self) -> String {
        let selection_summary = self
            .selection_evidence
            .as_ref()
            .map(format_evidence_summary)
            .or_else(|| self.selection_summary.map(format_selection_summary))
            .unwrap_or_else(|| "selection none".to_string());

        format!(
            "RawScope | pts {} ({}) | grid {}x{} | x {:.1}..{:.1} y {:.1}..{:.1} | max {} | {} | update {:.2}ms frame {:.2}ms | redraw {} | {} {} | wheel zoom, drag pan, right/shift-drag brush, Esc clear, R reset, 1-4 presets",
            self.render_diagnostics.point_count,
            self.preset.row_count_label(),
            self.render_diagnostics.grid_width,
            self.render_diagnostics.grid_height,
            self.viewport.x_range().min,
            self.viewport.x_range().max,
            self.viewport.y_range().min,
            self.viewport.y_range().max,
            self.render_diagnostics.max_bin_count,
            selection_summary,
            self.render_diagnostics
                .density_update_cpu_duration
                .as_secs_f64()
                * 1000.0,
            self.latest_frame_cpu_duration.as_secs_f64() * 1000.0,
            self.redraw_count,
            self.adapter_name,
            self.backend,
        )
    }
}

/// State displayed in the window-title diagnostics for timeline mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOverlayState {
    pub viewport: TimelineViewport,
    pub render_diagnostics: TimelineDensityRenderDiagnostics,
    pub redraw_count: u64,
    pub latest_frame_cpu_duration: Duration,
    pub adapter_name: String,
    pub backend: String,
}

impl TimelineOverlayState {
    /// Formats compact timeline diagnostics suitable for a winit window title.
    pub fn title(&self) -> String {
        format!(
            "RawScope | timeline events {} | lanes {} | grid {}x{} | time {}..{} full {}..{} | max {} | update {:.2}ms frame {:.2}ms | redraw {} | {} {} | wheel zoom time, drag pan time, R reset, --demo scatter for scatter view",
            self.render_diagnostics.event_count,
            self.render_diagnostics.lane_count,
            self.render_diagnostics.grid_width,
            self.render_diagnostics.grid_height,
            self.viewport.time_range().min,
            self.viewport.time_range().max,
            self.viewport.full_time_range().min,
            self.viewport.full_time_range().max,
            self.render_diagnostics.max_bin_count,
            self.render_diagnostics
                .density_update_cpu_duration
                .as_secs_f64()
                * 1000.0,
            self.latest_frame_cpu_duration.as_secs_f64() * 1000.0,
            self.redraw_count,
            self.adapter_name,
            self.backend,
        )
    }
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
        "evidence rows {} ({:.2}%) sample [{}] cats c:{} b:{} o:{} top {:?}",
        evidence.selected_row_count,
        evidence.selected_percentage,
        row_id_sample,
        evidence.category_counts.cluster,
        evidence.category_counts.background,
        evidence.category_counts.outlier,
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
        "sel {} ({:.2}%) brush x {:.1}..{:.1} y {:.1}..{:.1} data x {} y {} cats c:{} b:{} o:{} top {}",
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
        top_category,
    )
}

/// Returns the current screenshot capture policy for the demo.
pub fn screenshot_capture_note() -> &'static str {
    "Screenshot capture is skipped for Milestone 3D: native surface readback needs a small dedicated capture path, and adding image encoding dependencies would widen this hardening slice."
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rawscope_core::{F32Range, U64Range};
    use rawscope_render::{
        ScatterDensityRenderDiagnostics, ScatterViewport, TimelineDensityRenderDiagnostics,
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
    fn demo_mode_defaults_to_scatter() {
        assert_eq!(DemoMode::from_args(Vec::new()).unwrap(), DemoMode::Scatter);
    }

    #[test]
    fn demo_mode_parses_timeline_argument() {
        assert_eq!(
            DemoMode::from_args(["--demo".to_string(), "timeline".to_string()]).unwrap(),
            DemoMode::Timeline
        );
        assert_eq!(
            DemoMode::from_args(["--demo=scatter".to_string()]).unwrap(),
            DemoMode::Scatter
        );
    }

    #[test]
    fn demo_mode_rejects_unknown_argument() {
        assert!(DemoMode::from_args(["--timeline".to_string()]).is_err());
    }

    #[test]
    fn title_includes_core_overlay_fields() {
        let viewport = ScatterViewport::new(F32Range::new(0.0, 100.0), F32Range::new(0.0, 100.0));
        let overlay = DemoOverlayState {
            preset: PointCountPreset::default(),
            viewport,
            render_diagnostics: ScatterDensityRenderDiagnostics {
                point_count: 20_000,
                grid_width: 256,
                grid_height: 256,
                max_bin_count: 42,
                density_update_cpu_duration: Duration::from_millis(3),
            },
            selection_summary: None,
            selection_evidence: None,
            redraw_count: 7,
            latest_frame_cpu_duration: Duration::from_millis(1),
            adapter_name: "Adapter".to_string(),
            backend: "Backend".to_string(),
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
            render_diagnostics: TimelineDensityRenderDiagnostics {
                event_count: 20_000,
                lane_count: 8,
                grid_width: 256,
                grid_height: 8,
                time_range: rawscope_core::U64Range::new(0, 1_000),
                max_bin_count: 99,
                density_update_cpu_duration: Duration::from_millis(4),
            },
            redraw_count: 7,
            latest_frame_cpu_duration: Duration::from_millis(1),
            adapter_name: "Adapter".to_string(),
            backend: "Backend".to_string(),
        };

        let title = overlay.title();

        assert!(title.contains("timeline events 20000"));
        assert!(title.contains("lanes 8"));
        assert!(title.contains("grid 256x8"));
        assert!(title.contains("max 99"));
        assert!(title.contains("wheel zoom time"));
        assert!(title.contains("--demo scatter"));
    }
}
