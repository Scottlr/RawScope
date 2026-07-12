//! Shared lane-label normalization for CSV and Parquet adapters.

use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NormalizedLaneLabel(Arc<str>);

impl NormalizedLaneLabel {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let normalized = raw.trim();
        (!normalized.is_empty()).then(|| Self(Arc::from(normalized)))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::NormalizedLaneLabel;

    #[test]
    fn csv_and_parquet_decoded_spellings_share_trimmed_identity() {
        let csv = NormalizedLaneLabel::parse("  provider ").unwrap();
        let parquet = NormalizedLaneLabel::parse("provider").unwrap();
        assert_eq!(csv, parquet);
        assert_eq!(csv.as_str(), "provider");
        assert!(NormalizedLaneLabel::parse("   ").is_none());
    }
}
