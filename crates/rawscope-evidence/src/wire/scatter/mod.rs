//! Bounded dispatch for legacy scatter-selection artifacts.

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

use std::{error::Error, fmt, sync::Arc};

pub const SCATTER_ARTIFACT_KIND: &str = "scatter-selection-evidence";
pub const MAX_SCATTER_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyScatterEvidence {
    artifact_kind: Arc<str>,
    schema_version: u32,
    payload: Arc<[u8]>,
}

impl LegacyScatterEvidence {
    fn new(schema_version: u32, payload: &[u8]) -> Self {
        Self {
            artifact_kind: Arc::from(SCATTER_ARTIFACT_KIND),
            schema_version,
            payload: Arc::from(payload),
        }
    }

    pub fn artifact_kind(&self) -> &str {
        &self.artifact_kind
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn encoded_bytes(&self) -> &[u8] {
        &self.payload
    }

    pub fn encode(&self) -> Vec<u8> {
        self.payload.to_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedScatterEvidence {
    LegacyOnly(LegacyScatterEvidence),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterWireError {
    InputTooLarge {
        actual_bytes: usize,
        max_bytes: usize,
    },
    InvalidJson,
    RootMustBeObject,
    MissingArtifactKind,
    InvalidArtifactKind,
    MissingSchemaVersion,
    InvalidSchemaVersion,
    MissingRequiredField {
        version: u32,
        field: &'static str,
    },
    UnsupportedSchemaVersion(u32),
}

impl fmt::Display for ScatterWireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge {
                actual_bytes,
                max_bytes,
            } => write!(
                formatter,
                "scatter evidence is {actual_bytes} bytes; maximum is {max_bytes}"
            ),
            Self::InvalidJson => formatter.write_str("scatter evidence is not valid JSON"),
            Self::RootMustBeObject => {
                formatter.write_str("scatter evidence root must be an object")
            }
            Self::MissingArtifactKind => {
                formatter.write_str("scatter evidence is missing artifact_kind")
            }
            Self::InvalidArtifactKind => {
                formatter.write_str("scatter evidence has an unsupported artifact_kind")
            }
            Self::MissingSchemaVersion => {
                formatter.write_str("scatter evidence is missing schema_version")
            }
            Self::InvalidSchemaVersion => {
                formatter.write_str("scatter evidence schema_version is not an integer")
            }
            Self::MissingRequiredField { version, field } => write!(
                formatter,
                "scatter evidence v{version} is missing required field {field}"
            ),
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "scatter evidence schema version {version} is unsupported"
                )
            }
        }
    }
}

impl Error for ScatterWireError {}

pub fn decode(bytes: &[u8]) -> Result<DecodedScatterEvidence, ScatterWireError> {
    let schema_version = inspect(bytes)?;
    let evidence = match schema_version {
        v1::SCHEMA_VERSION => v1::decode(bytes).map(DecodedScatterEvidence::LegacyOnly),
        v2::SCHEMA_VERSION => v2::decode(bytes).map(DecodedScatterEvidence::LegacyOnly),
        v3::SCHEMA_VERSION => v3::decode(bytes).map(DecodedScatterEvidence::LegacyOnly),
        v4::SCHEMA_VERSION => v4::decode(bytes).map(DecodedScatterEvidence::LegacyOnly),
        v5::SCHEMA_VERSION => v5::decode(bytes).map(DecodedScatterEvidence::LegacyOnly),
        version => Err(ScatterWireError::UnsupportedSchemaVersion(version)),
    }?;
    Ok(evidence)
}

fn inspect(bytes: &[u8]) -> Result<u32, ScatterWireError> {
    if bytes.len() > MAX_SCATTER_ARTIFACT_BYTES {
        return Err(ScatterWireError::InputTooLarge {
            actual_bytes: bytes.len(),
            max_bytes: MAX_SCATTER_ARTIFACT_BYTES,
        });
    }
    let value = serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|_| ScatterWireError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(ScatterWireError::RootMustBeObject)?;
    let artifact_kind = object
        .get("artifact_kind")
        .and_then(serde_json::Value::as_str)
        .ok_or(ScatterWireError::MissingArtifactKind)?;
    if artifact_kind != SCATTER_ARTIFACT_KIND {
        return Err(ScatterWireError::InvalidArtifactKind);
    }
    let schema_version = object
        .get("schema_version")
        .ok_or(ScatterWireError::MissingSchemaVersion)?
        .as_u64()
        .ok_or(ScatterWireError::InvalidSchemaVersion)
        .and_then(|version| {
            u32::try_from(version).map_err(|_| ScatterWireError::InvalidSchemaVersion)
        })?;
    let required_field = if schema_version == 1 {
        "dataset_metadata"
    } else {
        "dataset_identity"
    };
    if !object.contains_key(required_field) {
        return Err(ScatterWireError::MissingRequiredField {
            version: schema_version,
            field: required_field,
        });
    }
    if !object.contains_key("selected_row_count") {
        return Err(ScatterWireError::MissingRequiredField {
            version: schema_version,
            field: "selected_row_count",
        });
    }
    Ok(schema_version)
}

fn decode_version(
    bytes: &[u8],
    expected_version: u32,
) -> Result<LegacyScatterEvidence, ScatterWireError> {
    let actual_version = inspect(bytes)?;
    if actual_version != expected_version {
        return Err(ScatterWireError::UnsupportedSchemaVersion(actual_version));
    }
    Ok(LegacyScatterEvidence::new(expected_version, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(version: u32) -> Vec<u8> {
        format!(r#"{{"artifact_kind":"{SCATTER_ARTIFACT_KIND}","schema_version":{version},"dataset_metadata":{{}},"dataset_identity":{{}},"selected_row_count":0}}"#)
            .into_bytes()
    }

    #[test]
    fn dispatches_all_supported_scatter_versions_as_legacy_only() {
        for version in 1..=5 {
            let decoded = decode(&artifact(version)).unwrap();
            let DecodedScatterEvidence::LegacyOnly(evidence) = decoded;
            assert_eq!(evidence.schema_version(), version);
            assert_eq!(evidence.encoded_bytes(), artifact(version));
        }
    }

    #[test]
    fn rejects_unknown_kind_version_and_oversized_input() {
        assert_eq!(
            decode(br#"{"artifact_kind":"other","schema_version":1}"#),
            Err(ScatterWireError::InvalidArtifactKind)
        );
        assert_eq!(
            decode(&artifact(6)),
            Err(ScatterWireError::UnsupportedSchemaVersion(6))
        );
        assert_eq!(
            decode(br#"{"artifact_kind":"scatter-selection-evidence","schema_version":2}"#),
            Err(ScatterWireError::MissingRequiredField {
                version: 2,
                field: "dataset_identity"
            })
        );
        assert_eq!(
            decode(&vec![b' '; MAX_SCATTER_ARTIFACT_BYTES + 1]),
            Err(ScatterWireError::InputTooLarge {
                actual_bytes: MAX_SCATTER_ARTIFACT_BYTES + 1,
                max_bytes: MAX_SCATTER_ARTIFACT_BYTES,
            })
        );
    }
}
