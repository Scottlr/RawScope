//! Raw signal, SpanFold duration, and sample-state timeline launch orchestration.

use std::{fs, io, process::Child};

use rawscope::{
    spanfold::SPANFOLD_SESSION_MANIFEST_FILE_NAME, LocalCsvSession, PreparedAdapterSession,
};
use rawscope_showcase_support::{Result, ShowcaseContext, ShowcaseError};

use crate::{
    analyse::{DURATION_SESSION_DIRECTORY_NAME, STATE_TIMELINE_SESSION_DIRECTORY_NAME},
    transform::NAB_TRANSFORMED_FILE_NAME,
};

const NAB_DATASET_ID: &str = "nab";
const RAW_SESSION_DIRECTORY_NAME: &str = "speed_7578-session";
const SAMPLE_INDEX_COLUMN: &str = "sample_index";
const VALUE_COLUMN: &str = "value";
const TIMESTAMP_COLUMN: &str = "timestamp";
const DISPLAY_NAME: &str = "NAB traffic speed 7578";

pub fn visualise(context: &ShowcaseContext) -> Result<()> {
    let source_path = context.paths.transformed.join(NAB_TRANSFORMED_FILE_NAME);
    let raw_session_directory = context.paths.analysis.join(RAW_SESSION_DIRECTORY_NAME);
    if raw_session_directory.exists() {
        fs::remove_dir_all(&raw_session_directory).map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "replace the prepared raw-value session",
                source,
            )
        })?;
    }

    let raw = LocalCsvSession::scatter(source_path, SAMPLE_INDEX_COLUMN, VALUE_COLUMN)
        .display_name(DISPLAY_NAME)
        .evidence_key(TIMESTAMP_COLUMN)
        .prepare(raw_session_directory)
        .map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "prepare the raw-value session", source)
        })?;
    let duration = prepared_spanfold_session(context, DURATION_SESSION_DIRECTORY_NAME)?;
    let timeline = prepared_spanfold_session(context, STATE_TIMELINE_SESSION_DIRECTORY_NAME)?;

    let mut children = Vec::with_capacity(3);
    launch(&raw, "raw traffic-speed scatter", &mut children)?;
    launch(
        &duration,
        "SpanFold interval-duration scatter",
        &mut children,
    )?;
    launch(&timeline, "SpanFold sample-state timeline", &mut children)?;
    wait_for_all(children)
}

fn prepared_spanfold_session(
    context: &ShowcaseContext,
    directory_name: &str,
) -> Result<PreparedAdapterSession> {
    let manifest = context
        .paths
        .analysis
        .join(directory_name)
        .join(SPANFOLD_SESSION_MANIFEST_FILE_NAME);
    PreparedAdapterSession::from_manifest(manifest, None).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "load a prepared SpanFold session", source)
    })
}

fn launch(
    prepared: &PreparedAdapterSession,
    label: &'static str,
    children: &mut Vec<(&'static str, Child)>,
) -> Result<()> {
    let child = prepared.launch().map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "launch a RawScope workbench", source)
    })?;
    children.push((label, child));
    Ok(())
}

fn wait_for_all(children: Vec<(&'static str, Child)>) -> Result<()> {
    for (label, mut child) in children {
        let status = child.wait().map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "wait for a RawScope workbench", source)
        })?;
        if !status.success() {
            return Err(ShowcaseError::workflow(
                NAB_DATASET_ID,
                "complete RawScope visualisation",
                io::Error::other(format!("{label} exited with {status}")),
            ));
        }
    }
    Ok(())
}
