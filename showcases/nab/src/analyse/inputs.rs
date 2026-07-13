//! Parsing and alignment of pinned NAB source, label, detector, and threshold inputs.

use std::{collections::HashMap, fs, path::Path};

use rawscope_showcase_support::{Result, ShowcaseError};
use serde::Deserialize;

use crate::fetch::{
    NAB_DETECTOR_FILE_NAME, NAB_LABELS_FILE_NAME, NAB_SOURCE_FILE_NAME, NAB_THRESHOLDS_FILE_NAME,
};

const NAB_DATASET_ID: &str = "nab";
const NAB_SERIES_PATH: &str = "realTraffic/speed_7578.csv";

#[derive(Debug, Deserialize)]
struct SourceRow {
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct DetectorRow {
    timestamp: String,
    anomaly_score: f64,
}

#[derive(Debug, Deserialize)]
struct Thresholds {
    numenta: DetectorThresholds,
}

#[derive(Debug, Deserialize)]
struct DetectorThresholds {
    standard: ThresholdProfile,
}

#[derive(Debug, Deserialize)]
struct ThresholdProfile {
    threshold: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SampleRange {
    pub(super) start: i64,
    pub(super) end: i64,
}

pub(super) struct NabInputs {
    pub(super) ground_truth: Vec<SampleRange>,
    pub(super) detector: Vec<SampleRange>,
}

pub(super) fn read_inputs(downloads: &Path) -> Result<NabInputs> {
    let timestamps = read_source_timestamps(&downloads.join(NAB_SOURCE_FILE_NAME))?;
    let ground_truth = read_ground_truth(&downloads.join(NAB_LABELS_FILE_NAME), &timestamps)?;
    let threshold = read_threshold(&downloads.join(NAB_THRESHOLDS_FILE_NAME))?;
    let detector = read_detector_windows(
        &downloads.join(NAB_DETECTOR_FILE_NAME),
        &timestamps,
        threshold,
    )?;
    Ok(NabInputs {
        ground_truth,
        detector,
    })
}

fn read_source_timestamps(path: &Path) -> Result<Vec<String>> {
    let mut reader = csv::Reader::from_path(path).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "open the pinned NAB series", source)
    })?;
    reader
        .deserialize::<SourceRow>()
        .map(|row| {
            row.map(|row| row.timestamp).map_err(|source| {
                ShowcaseError::workflow(NAB_DATASET_ID, "parse NAB source timestamps", source)
            })
        })
        .collect()
}

fn read_ground_truth(path: &Path, timestamps: &[String]) -> Result<Vec<SampleRange>> {
    let bytes = fs::read(path).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "read NAB combined windows", source)
    })?;
    let labels: HashMap<String, Vec<[String; 2]>> =
        serde_json::from_slice(&bytes).map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "parse NAB combined windows", source)
        })?;
    let windows = labels
        .get(NAB_SERIES_PATH)
        .ok_or_else(|| ShowcaseError::InvalidArtifact {
            dataset_id: NAB_DATASET_ID,
            path: path.to_path_buf(),
            reason: format!("missing label windows for {NAB_SERIES_PATH}"),
        })?;
    let sample_by_timestamp = timestamps
        .iter()
        .enumerate()
        .map(|(index, timestamp)| (timestamp.as_str(), index as i64))
        .collect::<HashMap<_, _>>();

    windows
        .iter()
        .map(|[start, end]| {
            let normalized_start = normalize_label_timestamp(start);
            let normalized_end = normalize_label_timestamp(end);
            match (
                sample_by_timestamp.get(normalized_start).copied(),
                sample_by_timestamp.get(normalized_end).copied(),
            ) {
                (Some(start), Some(inclusive_end)) => Ok(SampleRange {
                    start,
                    end: inclusive_end + 1,
                }),
                _ => Err(ShowcaseError::InvalidArtifact {
                    dataset_id: NAB_DATASET_ID,
                    path: path.to_path_buf(),
                    reason: format!(
                        "label boundary [{normalized_start}, {normalized_end}] is absent from the source series"
                    ),
                }),
            }
        })
        .collect()
}

fn normalize_label_timestamp(timestamp: &str) -> &str {
    timestamp.strip_suffix(".000000").unwrap_or(timestamp)
}

fn read_threshold(path: &Path) -> Result<f64> {
    let bytes = fs::read(path).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "read NAB detector thresholds", source)
    })?;
    let thresholds: Thresholds = serde_json::from_slice(&bytes).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "parse NAB detector thresholds", source)
    })?;
    Ok(thresholds.numenta.standard.threshold)
}

fn read_detector_windows(
    path: &Path,
    timestamps: &[String],
    threshold: f64,
) -> Result<Vec<SampleRange>> {
    let mut reader = csv::Reader::from_path(path).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "open the pinned Numenta result", source)
    })?;
    let rows = reader
        .deserialize::<DetectorRow>()
        .map(|row| {
            row.map_err(|source| {
                ShowcaseError::workflow(NAB_DATASET_ID, "parse Numenta result rows", source)
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if rows.len() != timestamps.len()
        || rows
            .iter()
            .zip(timestamps)
            .any(|(row, expected)| row.timestamp != *expected)
    {
        return Err(ShowcaseError::InvalidArtifact {
            dataset_id: NAB_DATASET_ID,
            path: path.to_path_buf(),
            reason: "detector timestamps do not align one-for-one with the source series"
                .to_string(),
        });
    }
    Ok(contiguous_ranges(
        rows.iter().map(|row| row.anomaly_score >= threshold),
    ))
}

fn contiguous_ranges(active: impl IntoIterator<Item = bool>) -> Vec<SampleRange> {
    let mut ranges = Vec::new();
    let mut current_start = None;
    let mut end = 0_i64;
    for (index, is_active) in active.into_iter().enumerate() {
        let index = index as i64;
        if is_active && current_start.is_none() {
            current_start = Some(index);
        } else if !is_active {
            if let Some(start) = current_start.take() {
                ranges.push(SampleRange { start, end: index });
            }
        }
        end = index + 1;
    }
    if let Some(start) = current_start {
        ranges.push(SampleRange { start, end });
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::{contiguous_ranges, SampleRange};

    #[test]
    fn threshold_hits_become_half_open_detector_windows() {
        let ranges = contiguous_ranges([false, true, true, false, true]);

        assert_eq!(
            ranges,
            [
                SampleRange { start: 1, end: 3 },
                SampleRange { start: 4, end: 5 }
            ]
        );
    }
}
