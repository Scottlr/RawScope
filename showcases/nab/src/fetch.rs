//! Pinned acquisition and integrity verification for the minimal NAB series.

use std::fs;

use rawscope_showcase_support::{Result, ShowcaseContext, ShowcaseError};
use sha2::{Digest, Sha256};

pub const NAB_SOURCE_FILE_NAME: &str = "speed_7578.csv";
const NAB_DATASET_ID: &str = "nab";
const NAB_SOURCE_URL: &str = "https://raw.githubusercontent.com/numenta/NAB/ea702d75cc2258d9d7dd35ca8e5e2539d71f3140/data/realTraffic/speed_7578.csv";
const NAB_SOURCE_SHA256: &str = "15ba994384730cb5a2e6818f17ded1430b40d92eb7a08e5b1a48a445c8c350f6";
const MAX_DOWNLOAD_SIZE_BYTES: u64 = 1_048_576;

pub fn fetch(context: &ShowcaseContext) -> Result<()> {
    let destination = context.paths.downloads.join(NAB_SOURCE_FILE_NAME);
    if destination.exists() {
        let bytes = fs::read(&destination).map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "read the existing download", source)
        })?;
        return verify_download(&destination, &bytes);
    }

    let mut response = ureq::get(NAB_SOURCE_URL).call().map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "download the pinned NAB series", source)
    })?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_SIZE_BYTES)
        .read_to_vec()
        .map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "read the NAB response body", source)
        })?;
    verify_download(&destination, &bytes)?;

    fs::create_dir_all(&context.paths.downloads).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "create the download directory", source)
    })?;
    fs::write(&destination, bytes).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "write the verified NAB download", source)
    })
}

fn verify_download(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual == NAB_SOURCE_SHA256 {
        return Ok(());
    }

    Err(ShowcaseError::InvalidArtifact {
        dataset_id: NAB_DATASET_ID,
        path: path.to_path_buf(),
        reason: format!("SHA-256 mismatch; expected {NAB_SOURCE_SHA256}, got {actual}"),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::verify_download;

    #[test]
    fn checksum_verification_rejects_unpinned_content() {
        let error = verify_download(Path::new("speed_7578.csv"), b"changed upstream content")
            .expect_err("changed content must not be persisted");

        assert!(error.to_string().contains("SHA-256 mismatch"));
    }
}
