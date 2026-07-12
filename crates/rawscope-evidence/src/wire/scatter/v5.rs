use super::{decode_version, LegacyScatterEvidence, ScatterWireError};

pub const SCHEMA_VERSION: u32 = 5;

pub fn decode(bytes: &[u8]) -> Result<LegacyScatterEvidence, ScatterWireError> {
    decode_version(bytes, SCHEMA_VERSION)
}
