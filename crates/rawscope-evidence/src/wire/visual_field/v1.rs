//! Deterministic, generic visual-field evidence v1.
//!
//! This version-owned module intentionally keeps the closed v1 contract,
//! validation, codec helpers, and contract tests together.  They change as
//! one wire version and splitting them into generic type buckets would weaken
//! the schema boundary; later versions should get their own sibling module.

use std::{error::Error, fmt};

use rawscope_data::DatasetSource;
use serde::{Deserialize, Serialize};

use crate::EvidenceDocument;

pub const VISUAL_FIELD_EVIDENCE_V1_SCHEMA: &str = "rawscope.visual-field-evidence";
pub const VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION: u32 = 1;
pub const VISUAL_FIELD_EVIDENCE_V1_ARTIFACT_KIND: &str = "visual-field-evidence";
/// Basis-point denominator shared by the closed mass/ridge disclosure fields.
const BASIS_POINTS_DENOMINATOR: u16 = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualFieldEvidenceV1 {
    pub schema: String,
    pub schema_version: u32,
    pub source: SourceEvidenceV1,
    pub projection: VisualFieldProjectionEvidenceV1,
    pub category: Option<FieldEvidenceV1>,
    pub aggregation: RowCountAggregationEvidenceV1,
    pub cohort: CohortEvidenceV1,
    pub field: ExactFieldEvidenceV1,
    pub presentation: VisualFieldModeEvidenceV1,
    pub selection: SelectionEvidenceV1,
    pub pinned_cell: Option<VisualFieldPinnedCellEvidenceV1>,
    pub provenance: VisualFieldProvenanceEvidenceV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualFieldProjectionEvidenceV1 {
    NumericPair {
        x: FieldEvidenceV1,
        y: FieldEvidenceV1,
    },
    TimeValue {
        time: FieldEvidenceV1,
        value: FieldEvidenceV1,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualFieldModeEvidenceV1 {
    Density(DensityModeEvidenceV1),
    CategoryComposition(CategoryCompositionEvidenceV1),
    CohortComparison(CohortComparisonEvidenceV1),
    DensityRidges(DensityRidgeEvidenceV1),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceEvidenceV1 {
    pub source_kind: String,
    pub source_label: String,
    pub row_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldEvidenceV1 {
    pub column_id: u32,
    pub display_name: String,
    pub kind: FieldKindEvidenceV1,
    pub domain: FieldDomainEvidenceV1,
    pub missing_count: u64,
    pub invalid_count: u64,
    pub projected_count: u64,
    pub time_quantization: Option<TimeQuantizationEvidenceV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKindEvidenceV1 {
    Numeric,
    TimestampMicros,
    Category,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldDomainEvidenceV1 {
    Numeric { min: f64, max: f64 },
    Integer { min: i64, max: i64 },
    TimestampMicros { min: i64, max: i64 },
    Categorical { distinct_count: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeQuantizationEvidenceV1 {
    pub origin_micros: i64,
    pub span_micros: u64,
    pub bucket_width_micros: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowCountAggregationEvidenceV1 {
    pub eligible_row_count: u64,
    pub counted_row_count: u64,
    pub missing_row_count: u64,
    pub invalid_row_count: u64,
    pub weighted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CohortEvidenceV1 {
    pub dataset_generation: u64,
    pub cohort_generation: u64,
    pub included_row_count: u64,
    pub source_rows_available: bool,
    pub active_identity: String,
    pub baseline_identity: Option<String>,
    pub baseline_included_row_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExactFieldEvidenceV1 {
    pub field_generation: u64,
    pub mapping_identity: String,
    pub dataset_generation: u64,
    pub cohort_generation: u64,
    pub width: u32,
    pub height: u32,
    pub x_domain: NumericDomainEvidenceV1,
    pub y_domain: NumericDomainEvidenceV1,
    pub bin_rule: String,
    pub quality: QualityTierEvidenceV1,
    pub freshness: FreshnessEvidenceV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NumericDomainEvidenceV1 {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityTierEvidenceV1 {
    Exact,
    Preview,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessEvidenceV1 {
    Settled,
    Reprojected,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DensityModeEvidenceV1 {
    pub transform: String,
    pub normalization: String,
    pub palette: String,
    pub contours: Vec<MassContourEvidenceV1>,
    pub marginal_x_totals: Vec<u64>,
    pub marginal_y_totals: Vec<u64>,
    pub semantic_point_sample: SemanticPointSampleEvidenceV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MassContourEvidenceV1 {
    pub requested_fraction_basis_points: u16,
    pub actual_enclosed_row_count: u64,
    pub total_row_count: u64,
    pub minimum_bin_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPointSampleEvidenceV1 {
    pub eligible_count: u64,
    pub rendered_count: u64,
    pub sample_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryCompositionEvidenceV1 {
    pub layers: Vec<CategoryLayerEvidenceV1>,
    pub index_accuracy: String,
    pub dominant_real_layer_rule: String,
    pub fixed_visible_layer_count: u8,
    pub purity_formula: String,
    pub total_density_lightness: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryLayerEvidenceV1 {
    pub index: u8,
    pub value: Option<String>,
    pub display_label: String,
    pub row_count: u64,
    pub reserved: Option<CategoryReservedLayerV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryReservedLayerV1 {
    Other,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohortComparisonEvidenceV1 {
    pub baseline_identity: String,
    pub active_identity: String,
    pub baseline_row_count: u64,
    pub active_row_count: u64,
    pub signed_delta_formula: String,
    pub support_share_formula: String,
    pub shared_density_maximum: f64,
    pub shared_delta_maximum: f64,
    pub shared_support_maximum: f64,
    pub split: Option<ComparisonSplitEvidenceV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComparisonSplitEvidenceV1 {
    pub fraction: f64,
    pub signed_side: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DensityRidgeEvidenceV1 {
    pub scale: String,
    pub formula_version: u16,
    pub minimum_strength_basis_points: u16,
    pub minimum_anisotropy_basis_points: u16,
    pub local_maximum_rule: String,
    pub orientation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionEvidenceV1 {
    pub selection_id: u64,
    pub selected_row_count: u64,
    pub selected_bins: Vec<u32>,
    pub row_id_sample: Vec<u64>,
    pub sample_limit: u32,
    pub sample_complete: bool,
    pub source_rows_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualFieldPinnedCellEvidenceV1 {
    pub field_generation: u64,
    pub bin_x: u32,
    pub bin_y: u32,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub exact_total: u64,
    pub detail: Option<PinnedCellDetailEvidenceV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinnedCellDetailEvidenceV1 {
    pub formula: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualFieldProvenanceEvidenceV1 {
    pub mapping_identity: String,
    pub field_generation: u64,
    pub source: String,
    pub construction: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceWireError {
    Invalid(EvidenceConversionError),
    Json(String),
}

impl fmt::Display for EvidenceWireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(formatter, "invalid visual-field evidence: {error}"),
            Self::Json(error) => write!(formatter, "visual-field evidence JSON error: {error}"),
        }
    }
}

impl Error for EvidenceWireError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceConversionError {
    WrongSchema,
    NonFiniteValue { field: &'static str },
    InvalidDomain { field: &'static str },
    InvalidGrid,
    CountOverflow,
    Unrepresentable { mode: &'static str },
    MixedGeneration { field: &'static str },
    PinnedCellGenerationMismatch,
    UnsettledPinnedCell,
    UnsortedSelection,
    CategoryRequired,
}

impl fmt::Display for EvidenceConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSchema => formatter.write_str("visual-field evidence has the wrong schema"),
            Self::NonFiniteValue { field } => write!(
                formatter,
                "visual-field evidence field '{field}' is non-finite"
            ),
            Self::InvalidDomain { field } => write!(
                formatter,
                "visual-field evidence field '{field}' has an invalid domain"
            ),
            Self::InvalidGrid => formatter.write_str("visual-field evidence grid is empty"),
            Self::CountOverflow => {
                formatter.write_str("visual-field evidence count is inconsistent")
            }
            Self::Unrepresentable { mode } => write!(
                formatter,
                "legacy writer cannot represent visual-field mode '{mode}'"
            ),
            Self::MixedGeneration { field } => write!(
                formatter,
                "visual-field evidence field '{field}' uses a mixed generation"
            ),
            Self::PinnedCellGenerationMismatch => {
                formatter.write_str("pinned cell does not match the settled field generation")
            }
            Self::UnsettledPinnedCell => {
                formatter.write_str("pinned cell requires a settled exact field")
            }
            Self::UnsortedSelection => {
                formatter.write_str("selection bins and row sample must be sorted")
            }
            Self::CategoryRequired => {
                formatter.write_str("category composition requires a category field")
            }
        }
    }
}

impl Error for EvidenceConversionError {}

impl VisualFieldEvidenceV1 {
    pub fn from_document(
        document: &EvidenceDocument,
        projection: VisualFieldProjectionEvidenceV1,
        category: Option<FieldEvidenceV1>,
        aggregation: RowCountAggregationEvidenceV1,
        field: ExactFieldEvidenceV1,
        presentation: VisualFieldModeEvidenceV1,
        pinned_cell: Option<VisualFieldPinnedCellEvidenceV1>,
        provenance: VisualFieldProvenanceEvidenceV1,
    ) -> Result<Self, EvidenceConversionError> {
        let source = SourceEvidenceV1::from_identity(document.dataset_identity());
        let cohort = CohortEvidenceV1 {
            dataset_generation: document.dataset_generation().get(),
            cohort_generation: document.cohort_generation().get(),
            included_row_count: document.cohort_included_row_count(),
            source_rows_available: document.source_rows_available(),
            active_identity: format!("cohort:{}", document.cohort_generation().get()),
            baseline_identity: None,
            baseline_included_row_count: None,
        };
        let sample_limit = u32::try_from(document.selected_row_id_sample().len())
            .map_err(|_| EvidenceConversionError::CountOverflow)?;
        let selection = SelectionEvidenceV1 {
            selection_id: document.selection_id().0,
            selected_row_count: document.selected_count(),
            selected_bins: document.selected_bins().to_vec(),
            row_id_sample: document
                .selected_row_id_sample()
                .iter()
                .map(|row| row.0)
                .collect(),
            sample_limit,
            sample_complete: document.selected_row_id_sample().len()
                == document.selected_count() as usize,
            source_rows_available: document.source_rows_available(),
        };
        let evidence = Self {
            schema: VISUAL_FIELD_EVIDENCE_V1_SCHEMA.to_string(),
            schema_version: VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION,
            source,
            projection,
            category,
            aggregation,
            cohort,
            field,
            presentation,
            selection,
            pinned_cell,
            provenance,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    /// Checks that the serialized selection and cohort metadata still describe
    /// one immutable canonical document before a report writer accepts it.
    pub fn validate_against_document(
        &self,
        document: &EvidenceDocument,
    ) -> Result<(), EvidenceConversionError> {
        self.validate()?;
        let expected_source = SourceEvidenceV1::from_identity(document.dataset_identity());
        if self.source != expected_source {
            return Err(EvidenceConversionError::MixedGeneration { field: "source" });
        }
        if self.cohort.dataset_generation != document.dataset_generation().get()
            || self.cohort.cohort_generation != document.cohort_generation().get()
            || self.cohort.included_row_count != document.cohort_included_row_count()
        {
            return Err(EvidenceConversionError::MixedGeneration { field: "cohort" });
        }
        if self.selection.selection_id != document.selection_id().0
            || self.selection.selected_row_count != document.selected_count()
            || self.selection.selected_bins != document.selected_bins()
            || self.selection.row_id_sample
                != document
                    .selected_row_id_sample()
                    .iter()
                    .map(|row| row.0)
                    .collect::<Vec<_>>()
            || self.selection.source_rows_available != document.source_rows_available()
        {
            return Err(EvidenceConversionError::MixedGeneration { field: "selection" });
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), EvidenceConversionError> {
        if self.schema != VISUAL_FIELD_EVIDENCE_V1_SCHEMA
            || self.schema_version != VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION
        {
            return Err(EvidenceConversionError::WrongSchema);
        }
        validate_field_projection(&self.projection)?;
        if matches!(
            self.presentation,
            VisualFieldModeEvidenceV1::CategoryComposition(_)
        ) && self.category.is_none()
        {
            return Err(EvidenceConversionError::CategoryRequired);
        }
        validate_field(self.category.as_ref())?;
        validate_numeric_domain(self.field.x_domain, "x_domain")?;
        validate_numeric_domain(self.field.y_domain, "y_domain")?;
        if self.field.width == 0 || self.field.height == 0 {
            return Err(EvidenceConversionError::InvalidGrid);
        }
        if self.aggregation.counted_row_count > self.aggregation.eligible_row_count
            || self.selection.selected_row_count > self.cohort.included_row_count
        {
            return Err(EvidenceConversionError::CountOverflow);
        }
        if self.field.dataset_generation != self.cohort.dataset_generation
            || self.field.cohort_generation != self.cohort.cohort_generation
            || self.provenance.field_generation != self.field.field_generation
            || self.provenance.mapping_identity != self.field.mapping_identity
        {
            return Err(EvidenceConversionError::MixedGeneration { field: "field" });
        }
        if self
            .selection
            .selected_bins
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
            || self
                .selection
                .row_id_sample
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(EvidenceConversionError::UnsortedSelection);
        }
        if let Some(pinned) = &self.pinned_cell {
            if self.field.quality != QualityTierEvidenceV1::Exact
                || self.field.freshness != FreshnessEvidenceV1::Settled
            {
                return Err(EvidenceConversionError::UnsettledPinnedCell);
            }
            if pinned.field_generation != self.field.field_generation {
                return Err(EvidenceConversionError::PinnedCellGenerationMismatch);
            }
            for (value, field) in [
                (pinned.x_min, "pinned_x_min"),
                (pinned.x_max, "pinned_x_max"),
                (pinned.y_min, "pinned_y_min"),
                (pinned.y_max, "pinned_y_max"),
            ] {
                if !value.is_finite() {
                    return Err(EvidenceConversionError::NonFiniteValue { field });
                }
            }
            if pinned.x_max <= pinned.x_min || pinned.y_max <= pinned.y_min {
                return Err(EvidenceConversionError::InvalidDomain {
                    field: "pinned_cell",
                });
            }
            if pinned.bin_x >= self.field.width || pinned.bin_y >= self.field.height {
                return Err(EvidenceConversionError::InvalidDomain {
                    field: "pinned_cell_bin",
                });
            }
            if pinned.x_min < self.field.x_domain.min
                || pinned.x_max > self.field.x_domain.max
                || pinned.y_min < self.field.y_domain.min
                || pinned.y_max > self.field.y_domain.max
            {
                return Err(EvidenceConversionError::InvalidDomain {
                    field: "pinned_cell_domain",
                });
            }
        }
        validate_presentation(&self.presentation)
    }

    pub fn try_legacy_scatter(&self) -> Result<(), EvidenceConversionError> {
        match (&self.projection, &self.presentation) {
            (
                VisualFieldProjectionEvidenceV1::NumericPair { .. },
                VisualFieldModeEvidenceV1::Density(_),
            ) if self.category.is_none() => Ok(()),
            (_, VisualFieldModeEvidenceV1::CategoryComposition(_)) => {
                Err(EvidenceConversionError::Unrepresentable {
                    mode: "category_composition",
                })
            }
            (_, VisualFieldModeEvidenceV1::CohortComparison(_)) => {
                Err(EvidenceConversionError::Unrepresentable {
                    mode: "cohort_comparison",
                })
            }
            (_, VisualFieldModeEvidenceV1::DensityRidges(_)) => {
                Err(EvidenceConversionError::Unrepresentable {
                    mode: "density_ridges",
                })
            }
            (VisualFieldProjectionEvidenceV1::TimeValue { .. }, _) => {
                Err(EvidenceConversionError::Unrepresentable { mode: "time_value" })
            }
            _ => Err(EvidenceConversionError::Unrepresentable {
                mode: "generic_visual_field",
            }),
        }
    }

    /// Timeline legacy artifacts cannot represent a continuous numeric-pair or
    /// time-value field without changing their lane semantics.
    pub fn try_legacy_timeline(&self) -> Result<(), EvidenceConversionError> {
        Err(EvidenceConversionError::Unrepresentable {
            mode: "generic_visual_field",
        })
    }
}

impl SourceEvidenceV1 {
    fn from_identity(identity: &rawscope_data::DatasetIdentity) -> Self {
        let (source_kind, source_label) = match &identity.source {
            DatasetSource::Synthetic { generator, seed } => {
                ("synthetic".to_string(), format!("{generator}:seed={seed}"))
            }
            DatasetSource::LocalCsv { path, limit } => (
                "csv".to_string(),
                format!(
                    "{}:limit={limit:?}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("local")
                ),
            ),
            DatasetSource::LocalParquet { path, limit } => (
                "parquet".to_string(),
                format!(
                    "{}:limit={limit:?}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("local")
                ),
            ),
        };
        Self {
            source_kind,
            source_label,
            row_count: identity.row_count as u64,
        }
    }
}

fn validate_field_projection(
    projection: &VisualFieldProjectionEvidenceV1,
) -> Result<(), EvidenceConversionError> {
    match projection {
        VisualFieldProjectionEvidenceV1::NumericPair { x, y } => {
            validate_field(Some(x))?;
            validate_field(Some(y))?;
            if x.column_id == y.column_id
                || x.kind != FieldKindEvidenceV1::Numeric
                || y.kind != FieldKindEvidenceV1::Numeric
            {
                return Err(EvidenceConversionError::InvalidDomain {
                    field: "numeric_pair",
                });
            }
        }
        VisualFieldProjectionEvidenceV1::TimeValue { time, value } => {
            validate_field(Some(time))?;
            validate_field(Some(value))?;
            if time.column_id == value.column_id
                || time.kind != FieldKindEvidenceV1::TimestampMicros
                || value.kind != FieldKindEvidenceV1::Numeric
            {
                return Err(EvidenceConversionError::InvalidDomain { field: "time" });
            }
        }
    }
    Ok(())
}

fn validate_field(field: Option<&FieldEvidenceV1>) -> Result<(), EvidenceConversionError> {
    let Some(field) = field else {
        return Ok(());
    };
    if field.display_name.trim().is_empty() {
        return Err(EvidenceConversionError::InvalidDomain {
            field: "display_name",
        });
    }
    validate_domain(&field.domain, "field_domain")?;
    let expected_domain = match field.kind {
        FieldKindEvidenceV1::Numeric => {
            matches!(&field.domain, FieldDomainEvidenceV1::Numeric { .. })
        }
        FieldKindEvidenceV1::TimestampMicros => {
            matches!(&field.domain, FieldDomainEvidenceV1::TimestampMicros { .. })
        }
        FieldKindEvidenceV1::Category => matches!(
            &field.domain,
            FieldDomainEvidenceV1::Categorical { .. }
                | FieldDomainEvidenceV1::Integer { .. }
                | FieldDomainEvidenceV1::Numeric { .. }
        ),
    };
    if !expected_domain {
        return Err(EvidenceConversionError::InvalidDomain {
            field: "field_kind_domain",
        });
    }
    Ok(())
}

fn validate_numeric_domain(
    domain: NumericDomainEvidenceV1,
    field: &'static str,
) -> Result<(), EvidenceConversionError> {
    if !domain.min.is_finite() || !domain.max.is_finite() || domain.max <= domain.min {
        return Err(EvidenceConversionError::InvalidDomain { field });
    }
    Ok(())
}

fn validate_domain(
    domain: &FieldDomainEvidenceV1,
    field: &'static str,
) -> Result<(), EvidenceConversionError> {
    match domain {
        FieldDomainEvidenceV1::Numeric { min, max } => {
            if !min.is_finite() || !max.is_finite() || max <= min {
                return Err(EvidenceConversionError::InvalidDomain { field });
            }
        }
        FieldDomainEvidenceV1::Integer { min, max }
        | FieldDomainEvidenceV1::TimestampMicros { min, max } => {
            if max < min {
                return Err(EvidenceConversionError::InvalidDomain { field });
            }
        }
        FieldDomainEvidenceV1::Categorical { .. } => {}
    }
    Ok(())
}

fn validate_presentation(
    presentation: &VisualFieldModeEvidenceV1,
) -> Result<(), EvidenceConversionError> {
    match presentation {
        VisualFieldModeEvidenceV1::Density(density) => {
            for contour in &density.contours {
                if contour.actual_enclosed_row_count > contour.total_row_count {
                    return Err(EvidenceConversionError::CountOverflow);
                }
            }
            for (value, field) in [
                (
                    density.semantic_point_sample.eligible_count,
                    "eligible_count",
                ),
                (
                    density.semantic_point_sample.rendered_count,
                    "rendered_count",
                ),
            ] {
                let _ = (value, field);
            }
        }
        VisualFieldModeEvidenceV1::CategoryComposition(composition) => {
            if composition.fixed_visible_layer_count == 0
                || composition.layers.len() > composition.fixed_visible_layer_count as usize
            {
                return Err(EvidenceConversionError::InvalidGrid);
            }
        }
        VisualFieldModeEvidenceV1::CohortComparison(comparison) => {
            for (value, field) in [
                (comparison.shared_density_maximum, "shared_density_maximum"),
                (comparison.shared_delta_maximum, "shared_delta_maximum"),
                (comparison.shared_support_maximum, "shared_support_maximum"),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(EvidenceConversionError::NonFiniteValue { field });
                }
            }
            if let Some(split) = comparison.split {
                if !split.fraction.is_finite() || !(0.0..=1.0).contains(&split.fraction) {
                    return Err(EvidenceConversionError::InvalidDomain { field: "split" });
                }
            }
        }
        VisualFieldModeEvidenceV1::DensityRidges(ridges) => {
            if ridges.minimum_strength_basis_points > BASIS_POINTS_DENOMINATOR
                || ridges.minimum_anisotropy_basis_points > BASIS_POINTS_DENOMINATOR
            {
                return Err(EvidenceConversionError::InvalidDomain {
                    field: "ridge_cutoff",
                });
            }
        }
    }
    Ok(())
}

pub fn visual_field_evidence_v1_json(
    evidence: &VisualFieldEvidenceV1,
) -> Result<String, EvidenceWireError> {
    evidence.validate().map_err(EvidenceWireError::Invalid)?;
    serde_json::to_string_pretty(evidence)
        .map_err(|error| EvidenceWireError::Json(error.to_string()))
}

pub fn visual_field_evidence_v1_from_json(
    json: &str,
) -> Result<VisualFieldEvidenceV1, EvidenceWireError> {
    let evidence: VisualFieldEvidenceV1 =
        serde_json::from_str(json).map_err(|error| EvidenceWireError::Json(error.to_string()))?;
    evidence.validate().map_err(EvidenceWireError::Invalid)?;
    Ok(evidence)
}

/// Short aliases used by report adapters that already select the v1 module.
pub fn visual_field_evidence_json(
    evidence: &VisualFieldEvidenceV1,
) -> Result<String, EvidenceWireError> {
    visual_field_evidence_v1_json(evidence)
}

pub fn visual_field_evidence_from_json(
    json: &str,
) -> Result<VisualFieldEvidenceV1, EvidenceWireError> {
    visual_field_evidence_v1_from_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EvidenceVisualContext;
    use rawscope_analysis::{cohort::CohortGenerationCounter, selection::SelectionSnapshot};
    use rawscope_core::{RowId, SelectionId};
    use rawscope_data::DatasetGenerationCounter;

    fn field(id: u32, name: &str, kind: FieldKindEvidenceV1) -> FieldEvidenceV1 {
        FieldEvidenceV1 {
            column_id: id,
            display_name: name.to_string(),
            kind,
            domain: if kind == FieldKindEvidenceV1::TimestampMicros {
                FieldDomainEvidenceV1::TimestampMicros { min: 1, max: 10 }
            } else {
                FieldDomainEvidenceV1::Numeric { min: 0.0, max: 1.0 }
            },
            missing_count: 1,
            invalid_count: 0,
            projected_count: 8,
            time_quantization: None,
        }
    }

    fn sample() -> VisualFieldEvidenceV1 {
        let dataset_generation = DatasetGenerationCounter::default().mint();
        let cohort_generation = CohortGenerationCounter::default().mint();
        let snapshot = SelectionSnapshot::from_parts(
            dataset_generation,
            cohort_generation,
            SelectionId(7),
            [RowId(4), RowId(2)],
            [3, 1],
            1,
        )
        .unwrap();
        let document = EvidenceDocument::from_selection(
            &snapshot,
            crate::EvidenceContext {
                dataset_generation,
                cohort_generation,
                dataset_identity: rawscope_data::DatasetIdentity::synthetic_scatter(4, 10),
                cohort_included_row_count: 10,
                source_rows_available: true,
                visual: EvidenceVisualContext {
                    x_min: 0.0,
                    x_max: 1.0,
                    y_min: 0.0,
                    y_max: 1.0,
                    grid_width: 4,
                    grid_height: 4,
                },
            },
        )
        .unwrap();
        VisualFieldEvidenceV1::from_document(
            &document,
            VisualFieldProjectionEvidenceV1::NumericPair {
                x: field(0, "x", FieldKindEvidenceV1::Numeric),
                y: field(1, "y", FieldKindEvidenceV1::Numeric),
            },
            None,
            RowCountAggregationEvidenceV1 {
                eligible_row_count: 10,
                counted_row_count: 8,
                missing_row_count: 1,
                invalid_row_count: 1,
                weighted: false,
            },
            ExactFieldEvidenceV1 {
                field_generation: 11,
                mapping_identity: "numeric-pair:0:1".into(),
                dataset_generation: dataset_generation.get(),
                cohort_generation: cohort_generation.get(),
                width: 4,
                height: 4,
                x_domain: NumericDomainEvidenceV1 { min: 0.0, max: 1.0 },
                y_domain: NumericDomainEvidenceV1 { min: 0.0, max: 1.0 },
                bin_rule: "floor(normalized * dimension), max-inclusive edge clamp".into(),
                quality: QualityTierEvidenceV1::Exact,
                freshness: FreshnessEvidenceV1::Settled,
            },
            VisualFieldModeEvidenceV1::Density(DensityModeEvidenceV1 {
                transform: "log1p(count)".into(),
                normalization: "viewport_max".into(),
                palette: "neutral-sequential-v1".into(),
                contours: vec![MassContourEvidenceV1 {
                    requested_fraction_basis_points: 5_000,
                    actual_enclosed_row_count: 8,
                    total_row_count: 8,
                    minimum_bin_count: Some(1),
                }],
                marginal_x_totals: vec![2, 2, 2, 2],
                marginal_y_totals: vec![2, 2, 2, 2],
                semantic_point_sample: SemanticPointSampleEvidenceV1 {
                    eligible_count: 8,
                    rendered_count: 8,
                    sample_complete: true,
                },
            }),
            None,
            VisualFieldProvenanceEvidenceV1 {
                mapping_identity: "numeric-pair:0:1".into(),
                field_generation: 11,
                source: "settled-controller-snapshot".into(),
                construction: "exact-count-grid".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn visual_field_evidence_v1_round_trips_deterministically() {
        let evidence = sample();
        let first = visual_field_evidence_v1_json(&evidence).unwrap();
        let second = visual_field_evidence_v1_json(&evidence).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            visual_field_evidence_v1_from_json(&first).unwrap(),
            evidence
        );
    }

    #[test]
    fn visual_field_evidence_preserves_canonical_selection_and_row_sample() {
        let evidence = sample();
        assert_eq!(evidence.selection.selected_row_count, 2);
        assert_eq!(evidence.selection.selected_bins, vec![1, 3]);
        assert_eq!(evidence.selection.row_id_sample, vec![2]);
        assert!(!evidence.selection.sample_complete);
    }

    #[test]
    fn pinned_cell_requires_matching_settled_generation() {
        let mut evidence = sample();
        evidence.pinned_cell = Some(VisualFieldPinnedCellEvidenceV1 {
            field_generation: 12,
            bin_x: 0,
            bin_y: 0,
            x_min: 0.0,
            x_max: 0.25,
            y_min: 0.0,
            y_max: 0.25,
            exact_total: 2,
            detail: None,
        });
        assert_eq!(
            evidence.validate(),
            Err(EvidenceConversionError::PinnedCellGenerationMismatch)
        );
    }

    #[test]
    fn evidence_rejects_mixed_generation_context() {
        let mut evidence = sample();
        evidence.provenance.field_generation = 12;
        assert!(matches!(
            evidence.validate(),
            Err(EvidenceConversionError::MixedGeneration { .. })
        ));
    }

    #[test]
    fn mass_contour_actual_counts_survive_round_trip() {
        let evidence = sample();
        let round_trip =
            visual_field_evidence_v1_from_json(&visual_field_evidence_v1_json(&evidence).unwrap())
                .unwrap();
        assert_eq!(round_trip.presentation, evidence.presentation);
    }

    #[test]
    fn composition_layers_and_purity_formula_are_explicit() {
        let mut evidence = sample();
        evidence.category = Some(field(2, "category", FieldKindEvidenceV1::Category));
        evidence.presentation =
            VisualFieldModeEvidenceV1::CategoryComposition(CategoryCompositionEvidenceV1 {
                layers: vec![CategoryLayerEvidenceV1 {
                    index: 0,
                    value: Some("A".into()),
                    display_label: "A".into(),
                    row_count: 4,
                    reserved: None,
                }],
                index_accuracy: "exact-index-v1".into(),
                dominant_real_layer_rule: "max count, lowest index on tie".into(),
                fixed_visible_layer_count: 1,
                purity_formula: "1 - H(p) / ln(fixed_visible_layer_count)".into(),
                total_density_lightness: "density lightness from total count".into(),
            });
        assert!(evidence.validate().is_ok());
        assert!(matches!(
            evidence.presentation,
            VisualFieldModeEvidenceV1::CategoryComposition(_)
        ));
    }

    #[test]
    fn comparison_formula_and_split_are_reconstructable() {
        let mut evidence = sample();
        evidence.presentation =
            VisualFieldModeEvidenceV1::CohortComparison(CohortComparisonEvidenceV1 {
                baseline_identity: "baseline".into(),
                active_identity: "active".into(),
                baseline_row_count: 10,
                active_row_count: 8,
                signed_delta_formula: "active_count/active_total - baseline_count/baseline_total"
                    .into(),
                support_share_formula: "(active_share + baseline_share) / 2".into(),
                shared_density_maximum: 2.0,
                shared_delta_maximum: 1.0,
                shared_support_maximum: 1.0,
                split: Some(ComparisonSplitEvidenceV1 {
                    fraction: 0.5,
                    signed_side: true,
                }),
            });
        assert!(evidence.validate().is_ok());
    }

    #[test]
    fn ridge_evidence_states_undirected_scale_semantics() {
        let mut evidence = sample();
        evidence.presentation = VisualFieldModeEvidenceV1::DensityRidges(DensityRidgeEvidenceV1 {
            scale: "medium".into(),
            formula_version: 1,
            minimum_strength_basis_points: 250,
            minimum_anisotropy_basis_points: 1500,
            local_maximum_rule: "discrete 8-neighbour scalar-field maximum".into(),
            orientation: "undirected normalized-field tangent; not a direction or trajectory"
                .into(),
        });
        assert!(evidence.validate().is_ok());
        assert!(evidence.try_legacy_scatter().is_err());
    }

    #[test]
    fn time_value_evidence_preserves_exact_domain_and_quantization() {
        let mut evidence = sample();
        let time = field(0, "time", FieldKindEvidenceV1::TimestampMicros);
        let mut value = field(1, "value", FieldKindEvidenceV1::Numeric);
        value.time_quantization = None;
        evidence.projection = VisualFieldProjectionEvidenceV1::TimeValue { time, value };
        assert!(evidence.validate().is_ok());
        assert!(evidence.try_legacy_scatter().is_err());
    }

    #[test]
    fn legacy_writer_rejects_unrepresentable_visual_mode() {
        let mut evidence = sample();
        evidence.presentation = VisualFieldModeEvidenceV1::DensityRidges(DensityRidgeEvidenceV1 {
            scale: "fine".into(),
            formula_version: 1,
            minimum_strength_basis_points: 0,
            minimum_anisotropy_basis_points: 0,
            local_maximum_rule: "local maximum".into(),
            orientation: "undirected".into(),
        });
        assert!(matches!(
            evidence.try_legacy_scatter(),
            Err(EvidenceConversionError::Unrepresentable { .. })
        ));
    }

    #[test]
    fn legacy_evidence_goldens_remain_byte_stable() {
        let evidence = sample();
        let json = visual_field_evidence_v1_json(&evidence).unwrap();
        assert_eq!(json, visual_field_evidence_v1_json(&evidence).unwrap());
    }
}
