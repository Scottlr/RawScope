//! Workbench demo-mode and synthetic point-count helpers.

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

#[cfg(test)]
mod tests {
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
}
