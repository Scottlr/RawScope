//! Additive generic session v2 wire contract.

use serde::{Deserialize, Serialize};

use super::v1::{invalid, DatasetProfileId};
use super::{
    RawScopeSessionManifestV1, SessionDatasetV1, SessionManifestError, MAX_SESSION_ROW_LIMIT,
    RAWSCOPE_SESSION_ARTIFACT_KIND, RAWSCOPE_SESSION_SCHEMA_VERSION,
    RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
};

/// A version-neutral source description used by the additive v2 wire contract.
pub type SessionSource = SessionDatasetV1;

/// Stable source-column binding carried by session v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionColumnBinding(pub String);

impl SessionColumnBinding {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Optional profile hint carried by session v2. It never gates capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionProfileHint(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionViewV2 {
    NumericPair {
        x: SessionColumnBinding,
        y: SessionColumnBinding,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<SessionColumnBinding>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<SessionProfileHint>,
    },
    TimeValue {
        time: SessionColumnBinding,
        value: SessionColumnBinding,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<SessionColumnBinding>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<SessionProfileHint>,
    },
    TimelineLane {
        time: SessionColumnBinding,
        lane: SessionColumnBinding,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<SessionProfileHint>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawScopeSessionManifestV2 {
    pub artifact_kind: String,
    pub schema_version: u32,
    pub source: SessionSource,
    pub view: SessionViewV2,
}

/// Version-dispatched wire manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSessionManifest {
    V1(RawScopeSessionManifestV1),
    V2(RawScopeSessionManifestV2),
}

impl RawScopeSessionManifestV2 {
    pub fn validate_shape(&self) -> Result<(), SessionManifestError> {
        if self.artifact_kind != RAWSCOPE_SESSION_ARTIFACT_KIND {
            return Err(SessionManifestError::InvalidArtifactKind {
                expected: RAWSCOPE_SESSION_ARTIFACT_KIND,
                actual: self.artifact_kind.clone(),
            });
        }
        if self.schema_version != RAWSCOPE_SESSION_SCHEMA_VERSION_V2 {
            return Err(SessionManifestError::UnsupportedSchemaVersion {
                expected: RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
                actual: self.schema_version,
            });
        }
        validate_source(&self.source)?;
        match &self.view {
            SessionViewV2::NumericPair {
                x,
                y,
                category,
                profile,
            } => {
                validate_binding("view.x", x)?;
                validate_binding("view.y", y)?;
                validate_distinct(&[Some(x), Some(y), category.as_ref()])?;
                validate_profile(profile)?;
            }
            SessionViewV2::TimeValue {
                time,
                value,
                category,
                profile,
            } => {
                validate_binding("view.time", time)?;
                validate_binding("view.value", value)?;
                validate_distinct(&[Some(time), Some(value), category.as_ref()])?;
                validate_profile(profile)?;
            }
            SessionViewV2::TimelineLane {
                time,
                lane,
                profile,
            } => {
                validate_binding("view.time", time)?;
                validate_binding("view.lane", lane)?;
                validate_distinct(&[Some(time), Some(lane), None])?;
                validate_profile(profile)?;
            }
        }
        Ok(())
    }
}

fn validate_source(source: &SessionSource) -> Result<(), SessionManifestError> {
    if source.path.as_os_str().is_empty() {
        return Err(invalid("source.path", "must not be empty"));
    }
    if source
        .limit
        .is_some_and(|limit| limit == 0 || limit > MAX_SESSION_ROW_LIMIT)
    {
        return Err(invalid(
            "source.limit",
            "must be positive and within the schema maximum",
        ));
    }
    for (field, value) in [
        ("source.display_name", source.display_name.as_deref()),
        ("source.evidence_key", source.evidence_key.as_deref()),
    ] {
        if value.is_some_and(|value| value.trim().is_empty()) {
            return Err(invalid(field, "must not be blank when provided"));
        }
    }
    Ok(())
}

fn validate_binding(
    field: &'static str,
    binding: &SessionColumnBinding,
) -> Result<(), SessionManifestError> {
    if binding.0.trim().is_empty() {
        return Err(invalid(field, "must not be blank"));
    }
    Ok(())
}

fn validate_distinct(
    bindings: &[Option<&SessionColumnBinding>],
) -> Result<(), SessionManifestError> {
    for (index, binding) in bindings.iter().enumerate() {
        let Some(binding) = binding else { continue };
        if bindings[..index]
            .iter()
            .flatten()
            .any(|other| other.0 == binding.0)
        {
            return Err(invalid("view", "bindings must name distinct columns"));
        }
    }
    Ok(())
}

fn validate_profile(profile: &Option<SessionProfileHint>) -> Result<(), SessionManifestError> {
    if let Some(profile) = profile {
        if profile.0.trim().is_empty() {
            return Err(invalid("view.profile", "must not be blank"));
        }
        if DatasetProfileId::parse(&profile.0).is_err() {
            return Err(SessionManifestError::UnsupportedProfile {
                value: profile.0.clone(),
            });
        }
    }
    Ok(())
}

/// Parses either supported session wire version with explicit dispatch.
pub fn parse_session_manifest_versioned(
    json: &str,
) -> Result<ParsedSessionManifest, SessionManifestError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|source| SessionManifestError::JsonParse { source })?;
    let schema_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| invalid("schema_version", "must be an unsigned integer"))?;
    match schema_version {
        RAWSCOPE_SESSION_SCHEMA_VERSION => {
            super::v1::parse_session_manifest(json).map(ParsedSessionManifest::V1)
        }
        RAWSCOPE_SESSION_SCHEMA_VERSION_V2 => {
            let manifest: RawScopeSessionManifestV2 = serde_json::from_value(value)
                .map_err(|source| SessionManifestError::JsonParse { source })?;
            manifest.validate_shape()?;
            Ok(ParsedSessionManifest::V2(manifest))
        }
        actual => Err(SessionManifestError::UnsupportedSchemaVersions {
            actual,
            supported: &[
                RAWSCOPE_SESSION_SCHEMA_VERSION,
                RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
            ],
        }),
    }
}

pub fn session_manifest_json_v2(
    manifest: &RawScopeSessionManifestV2,
) -> Result<String, SessionManifestError> {
    manifest.validate_shape()?;
    serde_json::to_string_pretty(manifest)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .map_err(|source| SessionManifestError::JsonSerialize { source })
}
