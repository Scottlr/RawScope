//! NAB source parsing and RawScope scatter projection.

use std::{fs, io};

use rawscope_showcase_support::{Result, ShowcaseContext, ShowcaseError};
use serde::{Deserialize, Serialize};

use crate::fetch::NAB_SOURCE_FILE_NAME;

pub const NAB_TRANSFORMED_FILE_NAME: &str = "speed_7578.rawscope.csv";
const NAB_DATASET_ID: &str = "nab";
const NAB_SERIES_NAME: &str = "speed_7578";
const STAGING_FILE_NAME: &str = "speed_7578.rawscope.csv.tmp";

#[derive(Debug, Deserialize)]
struct NabSourceRow {
    timestamp: String,
    value: f64,
}

#[derive(Debug, Serialize)]
struct NabScatterRow {
    sample_index: u64,
    timestamp: String,
    value: f64,
    series: &'static str,
}

pub fn transform(context: &ShowcaseContext) -> Result<()> {
    let source_path = context.paths.downloads.join(NAB_SOURCE_FILE_NAME);
    let destination_path = context.paths.transformed.join(NAB_TRANSFORMED_FILE_NAME);
    let staging_path = context.paths.transformed.join(STAGING_FILE_NAME);
    fs::create_dir_all(&context.paths.transformed).map_err(|source| {
        ShowcaseError::workflow(
            NAB_DATASET_ID,
            "create the transformed-data directory",
            source,
        )
    })?;

    let source = fs::File::open(&source_path).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "open the downloaded NAB series", source)
    })?;
    let destination = fs::File::create(&staging_path).map_err(|source| {
        ShowcaseError::workflow(
            NAB_DATASET_ID,
            "create the staged RawScope projection",
            source,
        )
    })?;
    if let Err(source) = transform_rows(source, destination) {
        let _ = fs::remove_file(&staging_path);
        return Err(ShowcaseError::workflow(
            NAB_DATASET_ID,
            "transform NAB rows for RawScope",
            source,
        ));
    }
    if destination_path.exists() {
        fs::remove_file(&destination_path).map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "replace the previous RawScope projection",
                source,
            )
        })?;
    }
    fs::rename(staging_path, destination_path).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "transform NAB rows for RawScope", source)
    })
}

fn transform_rows(source: impl io::Read, destination: impl io::Write) -> csv::Result<()> {
    let mut reader = csv::Reader::from_reader(source);
    let mut writer = csv::Writer::from_writer(destination);
    for (sample_index, row) in reader.deserialize::<NabSourceRow>().enumerate() {
        let row = row?;
        writer.serialize(NabScatterRow {
            sample_index: sample_index as u64,
            timestamp: row.timestamp,
            value: row.value,
            series: NAB_SERIES_NAME,
        })?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::transform_rows;

    #[test]
    fn transform_preserves_evidence_and_adds_ordered_scatter_coordinates() {
        let source = b"timestamp,value\n2015-09-08 11:39:00,73\n2015-09-08 11:44:00,62\n";
        let mut destination = Vec::new();

        transform_rows(source.as_slice(), &mut destination).unwrap();

        assert_eq!(
            String::from_utf8(destination).unwrap(),
            "sample_index,timestamp,value,series\n0,2015-09-08 11:39:00,73.0,speed_7578\n1,2015-09-08 11:44:00,62.0,speed_7578\n"
        );
    }
}
