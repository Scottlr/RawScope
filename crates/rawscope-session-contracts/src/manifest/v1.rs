use std::{error::Error, fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

pub const RAWSCOPE_SESSION_ARTIFACT_KIND: &str = "rawscope.session";
pub const RAWSCOPE_SESSION_SCHEMA_VERSION: u32 = 1;
pub const RAWSCOPE_SESSION_SCHEMA_VERSION_V2: u32 = 2;
pub const MAX_SESSION_MANIFEST_BYTES: u64 = 1_048_576;
pub const MAX_SESSION_ROW_LIMIT: u64 = 10_000_000_000;

#[derive(Debug)]
pub enum SessionManifestError {
    JsonParse {
        source: serde_json::Error,
    },
    JsonSerialize {
        source: serde_json::Error,
    },
    InvalidArtifactKind {
        expected: &'static str,
        actual: String,
    },
    UnsupportedSchemaVersion {
        expected: u32,
        actual: u32,
    },
    UnsupportedSchemaVersions {
        actual: u32,
        supported: &'static [u32],
    },
    InvalidField {
        field: &'static str,
        reason: String,
    },
    UnsupportedProfile {
        value: String,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
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
    ManifestTooLarge {
        path: PathBuf,
        max_bytes: u64,
    },
}

impl fmt::Display for SessionManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonParse { source } => write!(f, "invalid RawScope session JSON: {source}"),
            Self::JsonSerialize { source } => {
                write!(f, "failed to serialize RawScope session JSON: {source}")
            }
            Self::InvalidArtifactKind { expected, actual } => write!(
                f,
                "invalid session artifact kind '{actual}'; expected '{expected}'"
            ),
            Self::UnsupportedSchemaVersion { expected, actual } => write!(
                f,
                "unsupported RawScope session schema version {actual}; expected {expected}"
            ),
            Self::UnsupportedSchemaVersions { actual, supported } => write!(
                f,
                "unsupported RawScope session schema version {actual}; supported versions: {supported:?}"
            ),
            Self::InvalidField { field, reason } => {
                write!(f, "invalid session field '{field}': {reason}")
            }
            Self::UnsupportedProfile { value } => {
                write!(f, "unsupported dataset profile '{value}'")
            }
            Self::Io { path, source } => write!(
                f,
                "failed to access session path '{}': {source}",
                path.display()
            ),
            Self::UriPath { path } => write!(
                f,
                "session path '{}' uses a URI; RawScope sessions accept local paths only",
                path.display()
            ),
            Self::DatasetNotRegularFile { path } => write!(
                f,
                "session dataset path '{}' is not a regular file",
                path.display()
            ),
            Self::FormatExtensionMismatch { path, format } => write!(
                f,
                "session dataset '{}' does not have the expected .{} extension",
                path.display(),
                format.extension()
            ),
            Self::ManifestTooLarge { path, max_bytes } => write!(
                f,
                "session manifest '{}' exceeds the {max_bytes}-byte limit",
                path.display()
            ),
        }
    }
}

impl Error for SessionManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::JsonParse { source } | Self::JsonSerialize { source } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionDataFormat {
    Csv,
    Parquet,
}

impl SessionDataFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Parquet => "parquet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatasetProfileId {
    LichessGames,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetProfileParseError;

impl DatasetProfileId {
    pub const fn as_str(self) -> &'static str {
        "lichess-games"
    }
    pub fn parse(value: &str) -> Result<Self, DatasetProfileParseError> {
        if value.trim() == Self::LichessGames.as_str() {
            Ok(Self::LichessGames)
        } else {
            Err(DatasetProfileParseError)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDatasetV1 {
    pub path: PathBuf,
    pub format: SessionDataFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_key: Option<String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawScopeSessionManifestV1 {
    pub artifact_kind: String,
    pub schema_version: u32,
    pub dataset: SessionDatasetV1,
    pub view: SessionViewV1,
}

impl RawScopeSessionManifestV1 {
    pub fn validate_shape(&self) -> Result<(), SessionManifestError> {
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
        if self.dataset.path.as_os_str().is_empty() {
            return Err(invalid("dataset.path", "must not be empty"));
        }
        if self
            .dataset
            .limit
            .is_some_and(|limit| limit == 0 || limit > MAX_SESSION_ROW_LIMIT)
        {
            return Err(invalid(
                "dataset.limit",
                "must be positive and within the schema maximum",
            ));
        }
        for (field, value) in [
            ("dataset.display_name", self.dataset.display_name.as_deref()),
            ("dataset.evidence_key", self.dataset.evidence_key.as_deref()),
        ] {
            if value.is_some_and(|v| v.trim().is_empty()) {
                return Err(invalid(field, "must not be blank when provided"));
            }
        }
        match &self.view {
            SessionViewV1::Scatter { x, y, profile } => {
                validate_view("view.x", "view.y", x, y, profile)?
            }
            SessionViewV1::Timeline {
                time,
                lane,
                profile,
            } => validate_view("view.time", "view.lane", time, lane, profile)?,
        }
        Ok(())
    }
}

fn validate_view(
    x_field: &'static str,
    y_field: &'static str,
    x: &str,
    y: &str,
    profile: &Option<String>,
) -> Result<(), SessionManifestError> {
    if x.trim().is_empty() {
        return Err(invalid(x_field, "must not be blank"));
    }
    if y.trim().is_empty() {
        return Err(invalid(y_field, "must not be blank"));
    }
    if x.trim() == y.trim() {
        return Err(invalid("view", "bindings must differ"));
    }
    if let Some(profile) = profile {
        if profile.trim().is_empty() {
            return Err(invalid("view.profile", "must not be blank"));
        }
        if DatasetProfileId::parse(profile).is_err() {
            return Err(SessionManifestError::UnsupportedProfile {
                value: profile.clone(),
            });
        }
    }
    Ok(())
}

pub(crate) fn invalid(field: &'static str, reason: &str) -> SessionManifestError {
    SessionManifestError::InvalidField {
        field,
        reason: reason.to_string(),
    }
}

pub fn parse_session_manifest(
    json: &str,
) -> Result<RawScopeSessionManifestV1, SessionManifestError> {
    let manifest: RawScopeSessionManifestV1 =
        serde_json::from_str(json).map_err(|source| SessionManifestError::JsonParse { source })?;
    manifest.validate_shape()?;
    Ok(manifest)
}

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
