//! Portable source provenance with explicit sensitive-path opt-in.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintAlgorithm {
    Sha256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintCoverage {
    FullSource,
    HeaderAndSample,
    MetadataOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentFingerprintV1 {
    pub algorithm: FingerprintAlgorithm,
    pub coverage: FingerprintCoverage,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceProvenanceV1 {
    pub source_label: String,
    pub fingerprint: ContentFingerprintV1,
    pub display_path: Option<String>,
    pub absolute_path_sensitive: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProvenancePolicy {
    pub include_sensitive_absolute_path: bool,
}

impl EvidenceProvenanceV1 {
    pub fn new(
        source_label: impl Into<String>,
        fingerprint: ContentFingerprintV1,
        display_path: Option<String>,
        absolute_path: Option<String>,
        policy: ProvenancePolicy,
    ) -> Self {
        Self {
            source_label: source_label.into(),
            fingerprint,
            display_path,
            absolute_path_sensitive: policy
                .include_sensitive_absolute_path
                .then_some(absolute_path)
                .flatten(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint() -> ContentFingerprintV1 {
        ContentFingerprintV1 {
            algorithm: FingerprintAlgorithm::Sha256,
            coverage: FingerprintCoverage::HeaderAndSample,
            value: "abc".to_string(),
        }
    }

    #[test]
    fn provenance_redacts_absolute_paths_by_default() {
        let provenance = EvidenceProvenanceV1::new(
            "games.parquet",
            fingerprint(),
            Some("games.parquet".to_string()),
            Some("C:\\Users\\private\\games.parquet".to_string()),
            ProvenancePolicy::default(),
        );
        assert_eq!(provenance.absolute_path_sensitive, None);
        assert_eq!(provenance.display_path.as_deref(), Some("games.parquet"));
    }

    #[test]
    fn sensitive_path_requires_explicit_policy() {
        let provenance = EvidenceProvenanceV1::new(
            "games.parquet",
            fingerprint(),
            None,
            Some("C:\\Users\\private\\games.parquet".to_string()),
            ProvenancePolicy {
                include_sensitive_absolute_path: true,
            },
        );
        assert!(provenance.absolute_path_sensitive.is_some());
    }
}
