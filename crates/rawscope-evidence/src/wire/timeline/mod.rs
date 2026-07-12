//! Bounded dispatch for legacy timeline-selection artifacts.

pub mod v1;
pub mod v2;
pub mod v3;

use std::{error::Error, fmt, sync::Arc};

pub const TIMELINE_ARTIFACT_KIND: &str = "timeline-selection-evidence";
pub const MAX_TIMELINE_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyTimelineEvidence {
    artifact_kind: Arc<str>,
    schema_version: u32,
    payload: Arc<[u8]>,
}

impl LegacyTimelineEvidence {
    fn new(schema_version: u32, payload: &[u8]) -> Self {
        Self {
            artifact_kind: Arc::from(TIMELINE_ARTIFACT_KIND),
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
pub enum DecodedTimelineEvidence {
    LegacyOnly(LegacyTimelineEvidence),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineWireError {
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
    UnsupportedSchemaVersion(u32),
}

impl fmt::Display for TimelineWireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge {
                actual_bytes,
                max_bytes,
            } => write!(
                formatter,
                "timeline evidence is {actual_bytes} bytes; maximum is {max_bytes}"
            ),
            Self::InvalidJson => formatter.write_str("timeline evidence is not valid JSON"),
            Self::RootMustBeObject => {
                formatter.write_str("timeline evidence root must be an object")
            }
            Self::MissingArtifactKind => {
                formatter.write_str("timeline evidence is missing artifact_kind")
            }
            Self::InvalidArtifactKind => {
                formatter.write_str("timeline evidence has an unsupported artifact_kind")
            }
            Self::MissingSchemaVersion => {
                formatter.write_str("timeline evidence is missing schema_version")
            }
            Self::InvalidSchemaVersion => {
                formatter.write_str("timeline evidence schema_version is not an integer")
            }
            Self::UnsupportedSchemaVersion(version) => write!(
                formatter,
                "timeline evidence schema version {version} is unsupported"
            ),
        }
    }
}

impl Error for TimelineWireError {}

pub fn decode(bytes: &[u8]) -> Result<DecodedTimelineEvidence, TimelineWireError> {
    let version = inspect(bytes)?;
    let evidence = match version {
        v1::SCHEMA_VERSION => v1::decode(bytes).map(DecodedTimelineEvidence::LegacyOnly),
        v2::SCHEMA_VERSION => v2::decode(bytes).map(DecodedTimelineEvidence::LegacyOnly),
        v3::SCHEMA_VERSION => v3::decode(bytes).map(DecodedTimelineEvidence::LegacyOnly),
        version => Err(TimelineWireError::UnsupportedSchemaVersion(version)),
    }?;
    Ok(evidence)
}

fn inspect(bytes: &[u8]) -> Result<u32, TimelineWireError> {
    if bytes.len() > MAX_TIMELINE_ARTIFACT_BYTES {
        return Err(TimelineWireError::InputTooLarge {
            actual_bytes: bytes.len(),
            max_bytes: MAX_TIMELINE_ARTIFACT_BYTES,
        });
    }
    let value = serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|_| TimelineWireError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(TimelineWireError::RootMustBeObject)?;
    let kind = object
        .get("artifact_kind")
        .and_then(serde_json::Value::as_str)
        .ok_or(TimelineWireError::MissingArtifactKind)?;
    if kind != TIMELINE_ARTIFACT_KIND {
        return Err(TimelineWireError::InvalidArtifactKind);
    }
    object
        .get("schema_version")
        .ok_or(TimelineWireError::MissingSchemaVersion)?
        .as_u64()
        .ok_or(TimelineWireError::InvalidSchemaVersion)
        .and_then(|version| {
            u32::try_from(version).map_err(|_| TimelineWireError::InvalidSchemaVersion)
        })
}

fn decode_version(
    bytes: &[u8],
    expected_version: u32,
) -> Result<LegacyTimelineEvidence, TimelineWireError> {
    let actual = inspect(bytes)?;
    if actual != expected_version {
        return Err(TimelineWireError::UnsupportedSchemaVersion(actual));
    }
    Ok(LegacyTimelineEvidence::new(expected_version, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(version: u32) -> Vec<u8> {
        format!(r#"{{"artifact_kind":"{TIMELINE_ARTIFACT_KIND}","schema_version":{version}}}"#)
            .into_bytes()
    }

    #[test]
    fn dispatches_supported_timeline_versions_without_fabricating_fields() {
        for version in 1..=3 {
            let DecodedTimelineEvidence::LegacyOnly(evidence) = decode(&artifact(version)).unwrap();
            assert_eq!(evidence.schema_version(), version);
            assert_eq!(evidence.encode(), artifact(version));
        }
    }

    #[test]
    fn rejects_unknown_timeline_versions_and_oversized_input() {
        assert_eq!(
            decode(&artifact(4)),
            Err(TimelineWireError::UnsupportedSchemaVersion(4))
        );
        assert_eq!(
            decode(&vec![b' '; MAX_TIMELINE_ARTIFACT_BYTES + 1]),
            Err(TimelineWireError::InputTooLarge {
                actual_bytes: MAX_TIMELINE_ARTIFACT_BYTES + 1,
                max_bytes: MAX_TIMELINE_ARTIFACT_BYTES
            })
        );
    }
}
