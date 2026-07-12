//! Versioned local startup manifests shared by RawScope launchers.

mod manifest;
mod resolved_session;

pub use manifest::{
    parse_session_manifest, session_manifest_json, DatasetProfileId, RawScopeSessionManifestV1,
    SessionDataFormat, SessionDatasetV1, SessionManifestError, SessionViewV1,
    MAX_SESSION_MANIFEST_BYTES, MAX_SESSION_ROW_LIMIT, RAWSCOPE_SESSION_ARTIFACT_KIND,
    RAWSCOPE_SESSION_SCHEMA_VERSION,
};
pub use resolved_session::{
    load_session_manifest, ResolvedRawScopeSession, ResolvedSessionDataset, ResolvedSessionView,
};
