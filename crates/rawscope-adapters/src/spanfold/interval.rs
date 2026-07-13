//! Public configuration and rows for range-based SpanFold comparisons.

use serde::Serialize;
use spanfold::{ComparisonFinality, ComparisonResult, TemporalAxis};

use super::{error::SpanfoldAdapterError, interval_mapping};

const CORE_COMPARISON_FAMILIES: [SpanfoldIntervalFamily; 4] = [
    SpanfoldIntervalFamily::Overlap,
    SpanfoldIntervalFamily::Residual,
    SpanfoldIntervalFamily::Missing,
    SpanfoldIntervalFamily::Coverage,
];

const ALL_INTERVAL_FAMILIES: [SpanfoldIntervalFamily; 7] = [
    SpanfoldIntervalFamily::Overlap,
    SpanfoldIntervalFamily::Residual,
    SpanfoldIntervalFamily::Missing,
    SpanfoldIntervalFamily::Coverage,
    SpanfoldIntervalFamily::Gap,
    SpanfoldIntervalFamily::SymmetricDifference,
    SpanfoldIntervalFamily::Containment,
];

/// SpanFold result families that carry half-open temporal ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanfoldIntervalFamily {
    /// Time or processing range observed by both comparison sides.
    Overlap,
    /// Target-only range.
    Residual,
    /// Comparison-only range.
    Missing,
    /// Target range annotated with covered magnitude.
    Coverage,
    /// Empty range inside the observed comparison scope.
    Gap,
    /// Disagreement range annotated with its active side.
    SymmetricDifference,
    /// Target range annotated with containment status.
    Containment,
}

impl SpanfoldIntervalFamily {
    /// Stable CSV label for this family.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Overlap => "overlap",
            Self::Residual => "residual",
            Self::Missing => "missing",
            Self::Coverage => "coverage",
            Self::Gap => "gap",
            Self::SymmetricDifference => "symmetric_difference",
            Self::Containment => "containment",
        }
    }
}

/// Configures which range-based SpanFold result families become RawScope rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanfoldIntervalTransform {
    families: Vec<SpanfoldIntervalFamily>,
}

impl Default for SpanfoldIntervalTransform {
    fn default() -> Self {
        Self::core_comparison()
    }
}

impl SpanfoldIntervalTransform {
    /// Includes overlap, residual, missing, and coverage rows.
    #[must_use]
    pub fn core_comparison() -> Self {
        Self {
            families: CORE_COMPARISON_FAMILIES.to_vec(),
        }
    }

    /// Includes every SpanFold result family that represents a temporal range.
    #[must_use]
    pub fn all_intervals() -> Self {
        Self {
            families: ALL_INTERVAL_FAMILIES.to_vec(),
        }
    }

    /// Includes only the supplied interval families, preserving first-seen order.
    #[must_use]
    pub fn only(families: impl IntoIterator<Item = SpanfoldIntervalFamily>) -> Self {
        let mut selected = Vec::new();
        for family in families {
            if !selected.contains(&family) {
                selected.push(family);
            }
        }
        Self { families: selected }
    }

    /// Returns the selected interval families in output order.
    #[must_use]
    pub fn families(&self) -> &[SpanfoldIntervalFamily] {
        &self.families
    }

    /// Flattens selected SpanFold comparison rows into one evidence-preserving table.
    pub fn transform(
        &self,
        result: &ComparisonResult,
    ) -> Result<SpanfoldIntervalDataset, SpanfoldAdapterError> {
        interval_mapping::transform_comparison(result, &self.families)
    }
}

/// Rectangular interval table ready for CSV/session materialization.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanfoldIntervalDataset {
    plan_name: String,
    temporal_axis: TemporalAxis,
    clock: Option<String>,
    origin: i64,
    rows: Vec<SpanfoldIntervalRow>,
}

impl SpanfoldIntervalDataset {
    pub(super) fn new(
        plan_name: String,
        temporal_axis: TemporalAxis,
        clock: Option<String>,
        origin: i64,
        rows: Vec<SpanfoldIntervalRow>,
    ) -> Self {
        Self {
            plan_name,
            temporal_axis,
            clock,
            origin,
            rows,
        }
    }

    /// SpanFold comparison plan name retained on every output row.
    #[must_use]
    pub fn plan_name(&self) -> &str {
        &self.plan_name
    }

    /// Common temporal axis shared by every selected row.
    #[must_use]
    pub const fn temporal_axis(&self) -> TemporalAxis {
        self.temporal_axis
    }

    /// Common timestamp clock identity, when the result uses timestamp ticks.
    #[must_use]
    pub fn clock(&self) -> Option<&str> {
        self.clock.as_deref()
    }

    /// Exact minimum start magnitude used as the visual x-coordinate origin.
    #[must_use]
    pub const fn origin(&self) -> i64 {
        self.origin
    }

    /// Materialized interval rows in deterministic family/result order.
    #[must_use]
    pub fn rows(&self) -> &[SpanfoldIntervalRow] {
        &self.rows
    }
}

/// One flattened SpanFold interval with exact temporal and source-record evidence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpanfoldIntervalRow {
    /// Opaque deterministic identifier assigned by SpanFold.
    pub spanfold_row_id: String,
    /// SpanFold finality state for this row.
    pub spanfold_finality: ComparisonFinality,
    /// SpanFold's explanation of the current finality state.
    pub spanfold_finality_reason: String,
    /// SpanFold metadata version for this row.
    pub spanfold_row_version: u32,
    /// Prior SpanFold row identifier superseded by this version, when any.
    pub spanfold_supersedes_row_id: Option<String>,
    /// SpanFold comparator row family.
    pub row_family: SpanfoldIntervalFamily,
    /// Comparison plan name.
    pub plan_name: String,
    /// Window family name.
    pub window_name: String,
    /// Logical comparison key.
    pub key: String,
    /// Optional comparison partition.
    pub partition: Option<String>,
    /// Shared temporal axis label.
    pub temporal_axis: String,
    /// Timestamp clock identity, when any.
    pub clock: Option<String>,
    /// Exact origin subtracted from `start` for visual x coordinates.
    pub origin: i64,
    /// Exact inclusive start magnitude.
    pub start: i64,
    /// Exact exclusive end magnitude.
    pub end: i64,
    /// Visual x coordinate relative to `origin`.
    pub start_offset: f64,
    /// Exact interval duration (`end - start`).
    pub duration: i64,
    /// Coverage denominator for coverage rows.
    pub target_magnitude: Option<i64>,
    /// Covered magnitude for coverage rows.
    pub covered_magnitude: Option<i64>,
    /// Covered/target ratio for this coverage segment, when its denominator is positive.
    pub segment_coverage_ratio: Option<f64>,
    /// Active comparison side for symmetric-difference rows.
    pub side: Option<String>,
    /// Containment classification for containment rows.
    pub status: Option<String>,
    /// JSON array of exact target record identifiers retained in one CSV cell.
    pub target_record_ids: String,
    /// JSON array of exact comparison record identifiers retained in one CSV cell.
    pub against_record_ids: String,
}
