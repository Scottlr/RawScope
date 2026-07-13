//! RawScope session preparation and native workbench launch for the NAB series.

use std::{fs, io};

use rawscope::LocalCsvSession;
use rawscope_showcase_support::{Result, ShowcaseContext, ShowcaseError};

use crate::transform::NAB_TRANSFORMED_FILE_NAME;

const NAB_DATASET_ID: &str = "nab";
const SESSION_DIRECTORY_NAME: &str = "speed_7578-session";
const SAMPLE_INDEX_COLUMN: &str = "sample_index";
const VALUE_COLUMN: &str = "value";
const TIMESTAMP_COLUMN: &str = "timestamp";
const DISPLAY_NAME: &str = "NAB traffic speed 7578";

pub fn visualise(context: &ShowcaseContext) -> Result<()> {
    let source_path = context.paths.transformed.join(NAB_TRANSFORMED_FILE_NAME);
    let session_directory = context.paths.analysis.join(SESSION_DIRECTORY_NAME);
    if session_directory.exists() {
        fs::remove_dir_all(&session_directory).map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "replace the prepared RawScope session",
                source,
            )
        })?;
    }

    let prepared = LocalCsvSession::scatter(source_path, SAMPLE_INDEX_COLUMN, VALUE_COLUMN)
        .display_name(DISPLAY_NAME)
        .evidence_key(TIMESTAMP_COLUMN)
        .prepare(session_directory)
        .map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "prepare the RawScope session", source)
        })?;
    let mut child = prepared.launch().map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "launch the RawScope workbench", source)
    })?;
    let status = child.wait().map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "wait for the RawScope workbench", source)
    })?;
    if status.success() {
        return Ok(());
    }

    Err(ShowcaseError::workflow(
        NAB_DATASET_ID,
        "complete RawScope visualisation",
        io::Error::other(format!("workbench exited with {status}")),
    ))
}
