use super::{decode_version, LegacyTimelineEvidence, TimelineWireError};
pub const SCHEMA_VERSION: u32 = 3;
pub fn decode(bytes: &[u8]) -> Result<LegacyTimelineEvidence, TimelineWireError> {
    decode_version(bytes, SCHEMA_VERSION)
}
