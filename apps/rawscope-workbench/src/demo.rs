//! Workbench scatter-density demo controls and diagnostics helpers.

use std::time::Duration;

use rawscope_render::{ScatterDensityRenderDiagnostics, ScatterViewport, SelectedRegionSummary};

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
    pub redraw_count: u64,
    pub latest_frame_cpu_duration: Duration,
    pub adapter_name: String,
    pub backend: String,
}

impl DemoOverlayState {
    /// Formats compact diagnostics suitable for a winit window title.
    pub fn title(&self) -> String {
        let selection_summary = self
            .selection_summary
            .map(format_selection_summary)
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

    use rawscope_core::F32Range;
    use rawscope_render::{ScatterDensityRenderDiagnostics, ScatterViewport};

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
}
