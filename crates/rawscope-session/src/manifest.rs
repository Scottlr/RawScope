//! Serialized RawScope session v1 contract and format-independent validation.

use std::{error::Error, fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

pub const RAWSCOPE_SESSION_ARTIFACT_KIND: &str = "rawscope.session";
pub const RAWSCOPE_SESSION_SCHEMA_VERSION: u32 = 1;

/// Errors returned while parsing or validating a RawScope session manifest.
#[derive(Debug)]
pub enum SessionManifestError {
    JsonParse {
        source: serde_json::Error,
    },
    JsonSerialize {
        source: serde_json::Error,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidArtifactKind {
        expected: &'static str,
        actual: String,
    },
    UnsupportedSchemaVersion {
        expected: u32,
        actual: u32,
    },
    InvalidField {
        field: &'static str,
        reason: String,
    },
    UnsupportedProfile {
        value: String,
    },
    UriPath {
        path: PathBuf,
    },
    DatasetNotRegularFile {
        path: PathBuf,
    },
    FormatExtensionMismatch {
        path: PathBuf,
        format: SessionDataFormat,
    },
}

impl fmt::Display for SessionManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonParse { source } => {
                write!(formatter, "invalid RawScope session JSON: {source}")
            }
            Self::JsonSerialize { source } => {
                write!(
                    formatter,
                    "failed to serialize RawScope session JSON: {source}"
                )
            }
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to access session path '{}': {source}",
                    path.display()
                )
            }
            Self::InvalidArtifactKind { expected, actual } => write!(
                formatter,
                "invalid session artifact kind '{actual}'; expected '{expected}'"
            ),
            Self::UnsupportedSchemaVersion { expected, actual } => write!(
                formatter,
                "unsupported RawScope session schema version {actual}; expected {expected}"
            ),
            Self::InvalidField { field, reason } => {
                write!(formatter, "invalid session field '{field}': {reason}")
            }
            Self::UnsupportedProfile { value } => {
                write!(formatter, "unsupported dataset profile '{value}'")
            }
            Self::UriPath { path } => write!(
                formatter,
                "session path '{}' uses a URI; RawScope sessions accept local paths only",
                path.display()
            ),
            Self::DatasetNotRegularFile { path } => write!(
                formatter,
                "session dataset path '{}' is not a regular file",
                path.display()
            ),
            Self::FormatExtensionMismatch { path, format } => write!(
                formatter,
                "session dataset '{}' does not have the expected .{} extension",
                path.display(),
                format.extension()
            ),
        }
    }
}

impl Error for SessionManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::JsonParse { source } | Self::JsonSerialize { source } => Some(source),
            Self::Io { source, .. } => Some(source),
            Self::InvalidArtifactKind { .. }
            | Self::UnsupportedSchemaVersion { .. }
            | Self::InvalidField { .. }
            | Self::UnsupportedProfile { .. }
            | Self::UriPath { .. }
            | Self::DatasetNotRegularFile { .. }
            | Self::FormatExtensionMismatch { .. } => None,
        }
    }
}

/// Serialized session data format. The extension remains part of the v1
/// contract so launchers can detect accidental content/metadata mismatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionDataFormat {
    Csv,
    Parquet,
}

impl SessionDataFormat {
    pub(crate) const fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Parquet => "parquet",
        }
    }
}

/// Dataset reference and optional launch metadata in session schema v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDatasetV1 {
    pub path: PathBuf,
    pub format: SessionDataFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_key: Option<String>,
}

/// View binding encoded by session schema v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionViewV1 {
    Scatter {
        x: String,
        y: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<String>,
    },
    Timeline {
        time: String,
        lane: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<String>,
    },
}

/// Raw decoded session manifest before filesystem resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawScopeSessionManifestV1 {
    pub artifact_kind: String,
    pub schema_version: u32,
    pub dataset: SessionDatasetV1,
    pub view: SessionViewV1,
}

impl RawScopeSessionManifestV1 {
    pub(crate) fn validate_shape(&self) -> Result<(), SessionManifestError> {
        if self.artifact_kind != RAWSCOPE_SESSION_ARTIFACT_KIND {
            return Err(SessionManifestError::InvalidArtifactKind {
                expected: RAWSCOPE_SESSION_ARTIFACT_KIND,
                actual: self.artifact_kind.clone(),
            });
        }
        if self.schema_version != RAWSCOPE_SESSION_SCHEMA_VERSION {
            return Err(SessionManifestError::UnsupportedSchemaVersion {
                expected: RAWSCOPE_SESSION_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        validate_dataset_fields(&self.dataset)?;
        validate_view_fields(&self.view)?;
        Ok(())
    }
}

/// Parses and validates the format-independent portion of one session.
pub fn parse_session_manifest(
    json: &str,
) -> Result<RawScopeSessionManifestV1, SessionManifestError> {
    let manifest = serde_json::from_str::<RawScopeSessionManifestV1>(json)
        .map_err(|source| SessionManifestError::JsonParse { source })?;
    manifest.validate_shape()?;
    Ok(manifest)
}

/// Serializes a validated-compatible session manifest as readable canonical JSON.
pub fn session_manifest_json(
    manifest: &RawScopeSessionManifestV1,
) -> Result<String, SessionManifestError> {
    manifest.validate_shape()?;
    serde_json::to_string_pretty(manifest)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .map_err(|source| SessionManifestError::JsonSerialize { source })
}

fn validate_dataset_fields(dataset: &SessionDatasetV1) -> Result<(), SessionManifestError> {
    if dataset.path.as_os_str().is_empty() {
        return Err(invalid_field("dataset.path", "must not be empty"));
    }
    if dataset.limit == Some(0) {
        return Err(invalid_field("dataset.limit", "must be greater than zero"));
    }
    if dataset
        .display_name
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(invalid_field(
            "dataset.display_name",
            "must not be blank when provided",
        ));
    }
    if dataset
        .evidence_key
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(invalid_field(
            "dataset.evidence_key",
            "must not be blank when provided",
        ));
    }
    Ok(())
}

fn validate_view_fields(view: &SessionViewV1) -> Result<(), SessionManifestError> {
    match view {
        SessionViewV1::Scatter { x, y, profile } => {
            validate_binding("view.x", x)?;
            validate_binding("view.y", y)?;
            if x == y {
                return Err(invalid_field(
                    "view",
                    "scatter x and y bindings must differ",
                ));
            }
            validate_profile(profile.as_deref())?;
        }
        SessionViewV1::Timeline {
            time,
            lane,
            profile,
        } => {
            validate_binding("view.time", time)?;
            validate_binding("view.lane", lane)?;
            if time == lane {
                return Err(invalid_field(
                    "view",
                    "timeline time and lane bindings must differ",
                ));
            }
            validate_profile(profile.as_deref())?;
        }
    }
    Ok(())
}

fn validate_binding(field: &'static str, value: &str) -> Result<(), SessionManifestError> {
    if value.trim().is_empty() {
        return Err(invalid_field(field, "must not be blank"));
    }
    Ok(())
}

fn validate_profile(profile: Option<&str>) -> Result<(), SessionManifestError> {
    let Some(profile) = profile else {
        return Ok(());
    };
    if profile.trim().is_empty() {
        return Err(invalid_field("view.profile", "must not be blank"));
    }
    rawscope_data::parse_dataset_profile_id(profile).map_err(|_| {
        SessionManifestError::UnsupportedProfile {
            value: profile.to_string(),
        }
    })?;
    Ok(())
}

fn invalid_field(field: &'static str, reason: &str) -> SessionManifestError {
    SessionManifestError::InvalidField {
        field,
        reason: reason.to_string(),
    }
}
