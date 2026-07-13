//! Pinned acquisition and integrity verification for the NAB comparison inputs.

use std::{fs, path::Path};

use rawscope_showcase_support::{Result, ShowcaseContext, ShowcaseError};
use sha2::{Digest, Sha256};

pub const NAB_SOURCE_FILE_NAME: &str = "speed_7578.csv";
pub const NAB_LABELS_FILE_NAME: &str = "combined_windows.json";
pub const NAB_DETECTOR_FILE_NAME: &str = "numenta_speed_7578.csv";
pub const NAB_THRESHOLDS_FILE_NAME: &str = "thresholds.json";

const NAB_DATASET_ID: &str = "nab";
const NAB_REVISION_ROOT: &str =
    "https://raw.githubusercontent.com/numenta/NAB/ea702d75cc2258d9d7dd35ca8e5e2539d71f3140";
const MAX_DOWNLOAD_SIZE_BYTES: u64 = 1_048_576;

struct PinnedDownload {
    file_name: &'static str,
    relative_url: &'static str,
    sha256: &'static str,
}

const PINNED_DOWNLOADS: [PinnedDownload; 4] = [
    PinnedDownload {
        file_name: NAB_SOURCE_FILE_NAME,
        relative_url: "data/realTraffic/speed_7578.csv",
        sha256: "15ba994384730cb5a2e6818f17ded1430b40d92eb7a08e5b1a48a445c8c350f6",
    },
    PinnedDownload {
        file_name: NAB_LABELS_FILE_NAME,
        relative_url: "labels/combined_windows.json",
        sha256: "1e1fbc4601321aad8d0f8b3784c8134299379f68f6c1f7777565f8ffd57ab6b1",
    },
    PinnedDownload {
        file_name: NAB_DETECTOR_FILE_NAME,
        relative_url: "results/numenta/realTraffic/numenta_speed_7578.csv",
        sha256: "c7b42b8f95a7286d1dc06cb78451aed69ac7a542943a33dc6f001ce990905ca3",
    },
    PinnedDownload {
        file_name: NAB_THRESHOLDS_FILE_NAME,
        relative_url: "config/thresholds.json",
        sha256: "b0bd47854e33fede1a774b2c30d8e41c27e3f6cf1fc1f35a95b92beead315e69",
    },
];

pub fn fetch(context: &ShowcaseContext) -> Result<()> {
    fs::create_dir_all(&context.paths.downloads).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "create the download directory", source)
    })?;
    for pinned in &PINNED_DOWNLOADS {
        fetch_one(&context.paths.downloads, pinned)?;
    }
    Ok(())
}

fn fetch_one(downloads: &Path, pinned: &PinnedDownload) -> Result<()> {
    let destination = downloads.join(pinned.file_name);
    if destination.exists() {
        let bytes = fs::read(&destination).map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "read an existing pinned download", source)
        })?;
        return verify_download(&destination, &bytes, pinned.sha256);
    }

    let url = format!("{NAB_REVISION_ROOT}/{}", pinned.relative_url);
    let mut response = ureq::get(&url).call().map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "download a pinned NAB artifact", source)
    })?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_SIZE_BYTES)
        .read_to_vec()
        .map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "read a NAB response body", source)
        })?;
    verify_download(&destination, &bytes, pinned.sha256)?;
    fs::write(&destination, bytes).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "write a verified NAB download", source)
    })
}

fn verify_download(path: &Path, bytes: &[u8], expected_sha256: &str) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual == expected_sha256 {
        return Ok(());
    }

    Err(ShowcaseError::InvalidArtifact {
        dataset_id: NAB_DATASET_ID,
        path: path.to_path_buf(),
        reason: format!("SHA-256 mismatch; expected {expected_sha256}, got {actual}"),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::verify_download;

    #[test]
    fn checksum_verification_rejects_unpinned_content() {
        let error = verify_download(
            Path::new("speed_7578.csv"),
            b"changed upstream content",
            "expected",
        )
        .expect_err("changed content must not be persisted");

        assert!(error.to_string().contains("SHA-256 mismatch"));
    }
}
