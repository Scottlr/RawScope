//! Validated, portable report-bundle manifest contracts.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

pub const REPORT_BUNDLE_SCHEMA_VERSION_V2: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BundleRelativePath(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundlePathError {
    Empty,
    Absolute,
    ParentTraversal,
    AmbiguousSeparator,
    NotNormalized,
}

impl fmt::Display for BundlePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "bundle-relative path must not be empty",
            Self::Absolute => "bundle-relative path must not be absolute or prefixed",
            Self::ParentTraversal => "bundle-relative path must not contain parent traversal",
            Self::AmbiguousSeparator => "bundle-relative path must use '/' separators only",
            Self::NotNormalized => "bundle-relative path is not normalized",
        })
    }
}

impl Error for BundlePathError {}

impl BundleRelativePath {
    pub fn parse(value: impl Into<String>) -> Result<Self, BundlePathError> {
        let value = value.into();
        if value.is_empty() {
            return Err(BundlePathError::Empty);
        }
        if value.starts_with('/') || value.starts_with('\\') || value.contains(':') {
            return Err(BundlePathError::Absolute);
        }
        if value.contains('\\') {
            return Err(BundlePathError::AmbiguousSeparator);
        }
        let components: Vec<_> = value.split('/').collect();
        if components.iter().any(|component| component.is_empty()) {
            return Err(BundlePathError::NotNormalized);
        }
        if components.contains(&"..") {
            return Err(BundlePathError::ParentTraversal);
        }
        if components.contains(&".") {
            return Err(BundlePathError::NotNormalized);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceArtifactKind {
    Manifest,
    ScatterEvidence,
    TimelineEvidence,
    VisualFieldEvidence,
    Markdown,
    VisualContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentChecksumV1 {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleArtifactV2 {
    pub kind: EvidenceArtifactKind,
    pub schema_version: u32,
    pub relative_path: BundleRelativePath,
    pub length_bytes: u64,
    pub checksum: ContentChecksumV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifestV2 {
    pub bundle_schema_version: u32,
    pub artifacts: Vec<BundleArtifactV2>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleManifestError {
    UnsupportedSchemaVersion {
        actual: u32,
    },
    NoArtifacts,
    DuplicatePath {
        path: String,
    },
    InvalidPath {
        path: String,
        source: BundlePathError,
    },
    InvalidChecksum {
        path: String,
    },
}

impl fmt::Display for BundleManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { actual } => write!(
                formatter,
                "unsupported report bundle schema version {actual}"
            ),
            Self::NoArtifacts => {
                formatter.write_str("report bundle manifest must contain artifacts")
            }
            Self::DuplicatePath { path } => write!(
                formatter,
                "report bundle contains duplicate artifact path '{path}'"
            ),
            Self::InvalidPath { path, source } => write!(
                formatter,
                "invalid report bundle artifact path '{path}': {source}"
            ),
            Self::InvalidChecksum { path } => {
                write!(formatter, "artifact '{path}' has an empty checksum")
            }
        }
    }
}

impl Error for BundleManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPath { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl BundleManifestV2 {
    pub fn validate(&self) -> Result<(), BundleManifestError> {
        if self.bundle_schema_version != REPORT_BUNDLE_SCHEMA_VERSION_V2 {
            return Err(BundleManifestError::UnsupportedSchemaVersion {
                actual: self.bundle_schema_version,
            });
        }
        if self.artifacts.is_empty() {
            return Err(BundleManifestError::NoArtifacts);
        }
        let mut paths = std::collections::HashSet::with_capacity(self.artifacts.len());
        for artifact in &self.artifacts {
            let path = artifact.relative_path.as_str();
            BundleRelativePath::parse(path.to_owned()).map_err(|source| {
                BundleManifestError::InvalidPath {
                    path: path.to_owned(),
                    source,
                }
            })?;
            if !paths.insert(path) {
                return Err(BundleManifestError::DuplicatePath {
                    path: path.to_owned(),
                });
            }
            if !valid_sha256_checksum(&artifact.checksum) {
                return Err(BundleManifestError::InvalidChecksum {
                    path: path.to_owned(),
                });
            }
        }
        Ok(())
    }
}

fn valid_sha256_checksum(checksum: &ContentChecksumV1) -> bool {
    checksum.algorithm.eq_ignore_ascii_case("sha256")
        && checksum.value.len() == 64
        && checksum.value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_reject_escape_and_ambiguous_forms() {
        for value in [
            "../evidence.json",
            "nested/../../evidence.json",
            "nested\\evidence.json",
            "/evidence.json",
            "C:/evidence.json",
            "nested//evidence.json",
        ] {
            assert!(BundleRelativePath::parse(value).is_err(), "{value}");
        }
    }

    #[test]
    fn manifest_validation_rejects_duplicate_paths() {
        let path = BundleRelativePath::parse("evidence.json").unwrap();
        let artifact = BundleArtifactV2 {
            kind: EvidenceArtifactKind::ScatterEvidence,
            schema_version: 6,
            relative_path: path.clone(),
            length_bytes: 10,
            checksum: ContentChecksumV1 {
                algorithm: "sha256".into(),
                value: "0".repeat(64),
            },
        };
        let manifest = BundleManifestV2 {
            bundle_schema_version: 2,
            artifacts: vec![
                artifact.clone(),
                BundleArtifactV2 {
                    relative_path: path,
                    ..artifact
                },
            ],
        };
        assert!(matches!(
            manifest.validate(),
            Err(BundleManifestError::DuplicatePath { .. })
        ));
    }

    #[test]
    fn manifest_validation_rejects_non_sha256_checksums() {
        let manifest = BundleManifestV2 {
            bundle_schema_version: REPORT_BUNDLE_SCHEMA_VERSION_V2,
            artifacts: vec![BundleArtifactV2 {
                kind: EvidenceArtifactKind::ScatterEvidence,
                schema_version: 6,
                relative_path: BundleRelativePath::parse("evidence.json").unwrap(),
                length_bytes: 1,
                checksum: ContentChecksumV1 {
                    algorithm: "md5".into(),
                    value: "0".repeat(32),
                },
            }],
        };

        assert!(matches!(
            manifest.validate(),
            Err(BundleManifestError::InvalidChecksum { .. })
        ));
    }
}
