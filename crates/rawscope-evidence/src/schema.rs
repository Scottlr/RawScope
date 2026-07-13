//! Version-owned schema identities for new evidence artifacts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSchemaFamily {
    ScatterSelection,
    TimelineSelection,
    VisualField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceSchemaVersion {
    family: EvidenceSchemaFamily,
    version: u32,
}

pub const SCATTER_SELECTION_EVIDENCE_V6_SCHEMA_VERSION: u32 = 6;
pub const TIMELINE_SELECTION_EVIDENCE_V4_SCHEMA_VERSION: u32 = 4;
pub const VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION: u32 = 1;

impl EvidenceSchemaVersion {
    pub const fn scatter_v6() -> Self {
        Self {
            family: EvidenceSchemaFamily::ScatterSelection,
            version: SCATTER_SELECTION_EVIDENCE_V6_SCHEMA_VERSION,
        }
    }

    pub const fn timeline_v4() -> Self {
        Self {
            family: EvidenceSchemaFamily::TimelineSelection,
            version: TIMELINE_SELECTION_EVIDENCE_V4_SCHEMA_VERSION,
        }
    }

    pub const fn visual_field_v1() -> Self {
        Self {
            family: EvidenceSchemaFamily::VisualField,
            version: VISUAL_FIELD_EVIDENCE_V1_SCHEMA_VERSION,
        }
    }

    pub const fn family(self) -> EvidenceSchemaFamily {
        self.family
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_schema_identities_are_explicit_and_non_legacy() {
        assert_eq!(
            EvidenceSchemaVersion::scatter_v6().family(),
            EvidenceSchemaFamily::ScatterSelection
        );
        assert_eq!(EvidenceSchemaVersion::scatter_v6().version(), 6);
        assert_eq!(
            EvidenceSchemaVersion::timeline_v4().family(),
            EvidenceSchemaFamily::TimelineSelection
        );
        assert_eq!(EvidenceSchemaVersion::timeline_v4().version(), 4);
        assert_eq!(
            EvidenceSchemaVersion::visual_field_v1().family(),
            EvidenceSchemaFamily::VisualField
        );
        assert_eq!(EvidenceSchemaVersion::visual_field_v1().version(), 1);
    }
}
