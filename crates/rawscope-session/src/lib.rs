//! Versioned local startup manifests shared by RawScope launchers.

mod launcher;
mod manifest;
mod resolved_session;

pub use launcher::{
    WorkbenchLaunchError, WorkbenchLauncher, RAWSCOPE_WORKBENCH_COMMAND,
    RAWSCOPE_WORKBENCH_ENVIRONMENT_VARIABLE,
};
pub use manifest::{
    parse_session_manifest, parse_session_manifest_versioned, session_manifest_json,
    session_manifest_json_v2, DatasetProfileId, ParsedSessionManifest, RawScopeSessionManifestV1,
    RawScopeSessionManifestV2, SessionColumnBinding, SessionDataFormat, SessionDatasetV1,
    SessionManifestError, SessionProfileHint, SessionSource, SessionViewV1, SessionViewV2,
    MAX_SESSION_MANIFEST_BYTES, MAX_SESSION_ROW_LIMIT, RAWSCOPE_SESSION_ARTIFACT_KIND,
    RAWSCOPE_SESSION_SCHEMA_VERSION, RAWSCOPE_SESSION_SCHEMA_VERSION_V2,
};
pub use resolved_session::{
    load_session_manifest, ResolvedRawScopeSession, ResolvedSessionDataset, ResolvedSessionView,
};
