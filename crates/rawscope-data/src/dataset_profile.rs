//! Human-reviewed dataset profile contracts for local flagship datasets.

use std::{error::Error, fmt};

use crate::{LoadedColumnKind, LoadedColumnSchema};

const LICHESS_GAMES_PROFILE_VALUE: &str = "lichess-games";
const LICHESS_GAMES_DISPLAY_NAME: &str = "Lichess-style chess games";

const LICHESS_GAMES_REQUIRED_COLUMNS: [DatasetProfileColumn; 4] = [
    DatasetProfileColumn::new("created_at", LoadedColumnKind::Integer),
    DatasetProfileColumn::new("white_rating", LoadedColumnKind::Integer),
    DatasetProfileColumn::new("black_rating", LoadedColumnKind::Integer),
    DatasetProfileColumn::new("winner", LoadedColumnKind::String),
];

const LICHESS_GAMES_SCATTER_BINDING: ScatterDatasetProfileBinding =
    ScatterDatasetProfileBinding::new("white_rating", "black_rating");
const LICHESS_GAMES_TIMELINE_BINDING: TimelineDatasetProfileBinding =
    TimelineDatasetProfileBinding::new("created_at", "winner");

const LICHESS_GAMES_PROFILE: DatasetProfile = DatasetProfile {
    id: DatasetProfileId::LichessGames,
    display_name: LICHESS_GAMES_DISPLAY_NAME,
    required_columns: &LICHESS_GAMES_REQUIRED_COLUMNS,
    scatter_binding: Some(LICHESS_GAMES_SCATTER_BINDING),
    timeline_binding: Some(LICHESS_GAMES_TIMELINE_BINDING),
};

const SUPPORTED_DATASET_PROFILE_IDS: [DatasetProfileId; 1] = [DatasetProfileId::LichessGames];

/// Stable identifier for a human-reviewed local dataset profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatasetProfileId {
    LichessGames,
}

impl DatasetProfileId {
    /// Returns the stable CLI/runtime value for this profile.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LichessGames => LICHESS_GAMES_PROFILE_VALUE,
        }
    }
}

impl fmt::Display for DatasetProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One required column contract for a local dataset profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetProfileColumn {
    pub name: &'static str,
    pub kind: LoadedColumnKind,
}

impl DatasetProfileColumn {
    pub const fn new(name: &'static str, kind: LoadedColumnKind) -> Self {
        Self { name, kind }
    }
}

/// Default scatter binding supplied by a dataset profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterDatasetProfileBinding {
    pub x_column: &'static str,
    pub y_column: &'static str,
}

impl ScatterDatasetProfileBinding {
    pub const fn new(x_column: &'static str, y_column: &'static str) -> Self {
        Self { x_column, y_column }
    }
}

/// Default timeline binding supplied by a dataset profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineDatasetProfileBinding {
    pub time_column: &'static str,
    pub lane_column: &'static str,
}

impl TimelineDatasetProfileBinding {
    pub const fn new(time_column: &'static str, lane_column: &'static str) -> Self {
        Self {
            time_column,
            lane_column,
        }
    }
}

/// Human-reviewed binding and schema contract for a local dataset profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetProfile {
    pub id: DatasetProfileId,
    pub display_name: &'static str,
    pub required_columns: &'static [DatasetProfileColumn],
    pub scatter_binding: Option<ScatterDatasetProfileBinding>,
    pub timeline_binding: Option<TimelineDatasetProfileBinding>,
}

/// Parses one stable dataset profile identifier.
pub fn parse_dataset_profile_id(value: &str) -> Result<DatasetProfileId, DatasetProfileParseError> {
    match value {
        LICHESS_GAMES_PROFILE_VALUE => Ok(DatasetProfileId::LichessGames),
        _ => Err(DatasetProfileParseError {
            value: value.to_string(),
        }),
    }
}

/// Returns the supported stable dataset profile identifiers.
pub fn supported_dataset_profile_ids() -> &'static [DatasetProfileId] {
    &SUPPORTED_DATASET_PROFILE_IDS
}

/// Returns the contract for one dataset profile identifier.
pub fn dataset_profile(id: DatasetProfileId) -> &'static DatasetProfile {
    match id {
        DatasetProfileId::LichessGames => &LICHESS_GAMES_PROFILE,
    }
}

/// Validates that a loaded local schema satisfies one dataset profile contract.
pub fn validate_dataset_profile(
    profile: &DatasetProfile,
    available_columns: &[LoadedColumnSchema],
) -> Result<(), DatasetProfileValidationError> {
    let mut missing_columns = Vec::new();
    let mut type_mismatches = Vec::new();

    for required_column in profile.required_columns {
        let Some(available_column) = available_columns
            .iter()
            .find(|column| column.name == required_column.name)
        else {
            missing_columns.push(required_column.name.to_string());
            continue;
        };

        if available_column.kind != required_column.kind {
            type_mismatches.push(DatasetProfileValidationMismatch {
                column: required_column.name.to_string(),
                expected: required_column.kind,
                actual: available_column.kind,
            });
        }
    }

    if missing_columns.is_empty() && type_mismatches.is_empty() {
        return Ok(());
    }

    Err(DatasetProfileValidationError {
        profile_id: profile.id,
        missing_columns,
        type_mismatches,
    })
}

/// Parse failure for a stable dataset profile identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetProfileParseError {
    value: String,
}

impl fmt::Display for DatasetProfileParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let supported_ids = supported_dataset_profile_ids()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "unknown dataset profile '{}'; supported profiles: {supported_ids}",
            self.value
        )
    }
}

impl Error for DatasetProfileParseError {}

/// One required-column type mismatch found during profile validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetProfileValidationMismatch {
    pub column: String,
    pub expected: LoadedColumnKind,
    pub actual: LoadedColumnKind,
}

/// Validation failure for a dataset profile against a loaded local schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetProfileValidationError {
    pub profile_id: DatasetProfileId,
    pub missing_columns: Vec<String>,
    pub type_mismatches: Vec<DatasetProfileValidationMismatch>,
}

impl fmt::Display for DatasetProfileValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut issues = Vec::new();
        if !self.missing_columns.is_empty() {
            issues.push(format!(
                "missing required columns: {}",
                self.missing_columns.join(", ")
            ));
        }
        if !self.type_mismatches.is_empty() {
            issues.push(format!(
                "type mismatches: {}",
                self.type_mismatches
                    .iter()
                    .map(|mismatch| format!(
                        "{} expected {} but found {}",
                        mismatch.column, mismatch.expected, mismatch.actual
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        write!(
            f,
            "dataset profile '{}' is incompatible with the loaded schema: {}",
            self.profile_id,
            issues.join("; ")
        )
    }
}

impl Error for DatasetProfileValidationError {}
