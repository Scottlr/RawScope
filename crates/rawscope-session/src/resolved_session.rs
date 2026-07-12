//! Filesystem and dataset-profile resolution for session schema v1.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{
    parse_session_manifest, DatasetProfileId, RawScopeSessionManifestV1, SessionDataFormat,
    SessionDatasetV1, SessionManifestError, SessionViewV1, MAX_SESSION_MANIFEST_BYTES,
};

/// A session manifest after local paths and optional profile ids are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRawScopeSession {
    pub manifest_path: PathBuf,
    pub dataset: ResolvedSessionDataset,
    pub view: ResolvedSessionView,
}

/// Resolved dataset metadata for workbench startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSessionDataset {
    pub path: PathBuf,
    pub format: SessionDataFormat,
    pub display_name: Option<String>,
    pub limit: Option<u64>,
    pub evidence_key: Option<String>,
}

/// Resolved view binding for workbench startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedSessionView {
    Scatter {
        x: String,
        y: String,
        profile: Option<DatasetProfileId>,
    },
    Timeline {
        time: String,
        lane: String,
        profile: Option<DatasetProfileId>,
    },
}

/// Loads, validates, and resolves one local RawScope session manifest.
pub fn load_session_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<ResolvedRawScopeSession, SessionManifestError> {
    let requested_manifest_path = manifest_path.as_ref().to_path_buf();
    reject_uri_path(&requested_manifest_path)?;
    let manifest_path = canonical_file_path(&requested_manifest_path)?;
    let metadata = fs::metadata(&manifest_path).map_err(|source| SessionManifestError::Io {
        path: manifest_path.clone(),
        source,
    })?;
    if metadata.len() > MAX_SESSION_MANIFEST_BYTES {
        return Err(SessionManifestError::ManifestTooLarge {
            path: manifest_path,
            max_bytes: MAX_SESSION_MANIFEST_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity((metadata.len() as usize).saturating_add(1));
    fs::File::open(&manifest_path)
        .map_err(|source| SessionManifestError::Io {
            path: manifest_path.clone(),
            source,
        })?
        .take(MAX_SESSION_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| SessionManifestError::Io {
            path: manifest_path.clone(),
            source,
        })?;
    if bytes.len() as u64 > MAX_SESSION_MANIFEST_BYTES {
        return Err(SessionManifestError::ManifestTooLarge {
            path: manifest_path,
            max_bytes: MAX_SESSION_MANIFEST_BYTES,
        });
    }
    let json = String::from_utf8_lossy(&bytes);
    let manifest = parse_session_manifest(&json)?;
    resolve_manifest(manifest_path, manifest)
}

fn resolve_manifest(
    manifest_path: PathBuf,
    manifest: RawScopeSessionManifestV1,
) -> Result<ResolvedRawScopeSession, SessionManifestError> {
    let dataset_path = resolve_dataset_path(&manifest_path, &manifest.dataset)?;
    let dataset = ResolvedSessionDataset {
        path: dataset_path,
        format: manifest.dataset.format,
        display_name: manifest.dataset.display_name,
        limit: manifest.dataset.limit,
        evidence_key: manifest.dataset.evidence_key,
    };
    let view = resolve_view(manifest.view)?;
    Ok(ResolvedRawScopeSession {
        manifest_path,
        dataset,
        view,
    })
}

fn resolve_dataset_path(
    manifest_path: &Path,
    dataset: &SessionDatasetV1,
) -> Result<PathBuf, SessionManifestError> {
    reject_uri_path(&dataset.path)?;
    let manifest_directory = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let requested_path = if dataset.path.is_absolute() {
        dataset.path.clone()
    } else {
        manifest_directory.join(&dataset.path)
    };
    let resolved_path = canonical_file_path(&requested_path)?;
    let extension_matches = resolved_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(dataset.format.extension()));
    if !extension_matches {
        return Err(SessionManifestError::FormatExtensionMismatch {
            path: resolved_path,
            format: dataset.format,
        });
    }
    Ok(resolved_path)
}

fn resolve_view(view: SessionViewV1) -> Result<ResolvedSessionView, SessionManifestError> {
    match view {
        SessionViewV1::Scatter { x, y, profile } => Ok(ResolvedSessionView::Scatter {
            x,
            y,
            profile: resolve_profile(profile)?,
        }),
        SessionViewV1::Timeline {
            time,
            lane,
            profile,
        } => Ok(ResolvedSessionView::Timeline {
            time,
            lane,
            profile: resolve_profile(profile)?,
        }),
    }
}

fn resolve_profile(
    profile: Option<String>,
) -> Result<Option<DatasetProfileId>, SessionManifestError> {
    profile
        .map(|value| {
            DatasetProfileId::parse(&value)
                .map_err(|_| SessionManifestError::UnsupportedProfile { value })
        })
        .transpose()
}

fn canonical_file_path(path: &Path) -> Result<PathBuf, SessionManifestError> {
    let metadata = fs::metadata(path).map_err(|source| SessionManifestError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(SessionManifestError::DatasetNotRegularFile {
            path: path.to_path_buf(),
        });
    }
    fs::canonicalize(path).map_err(|source| SessionManifestError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn reject_uri_path(path: &Path) -> Result<(), SessionManifestError> {
    let path_text = path.to_string_lossy();
    if path_text.contains("://") {
        return Err(SessionManifestError::UriPath {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}
