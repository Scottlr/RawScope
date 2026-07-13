//! Sample-level comparison-state timeline derived from SpanFold interval evidence.

use std::{fs, path::Path};

use rawscope::{
    spanfold::{SpanfoldIntervalDataset, SpanfoldIntervalFamily, SpanfoldIntervalRow},
    LocalCsvSession,
};
use rawscope_showcase_support::{Result, ShowcaseError};
use serde::Serialize;

use super::inputs::NabSample;

pub(super) const STATE_TIMELINE_SESSION_DIRECTORY_NAME: &str = "spanfold-state-timeline-session";
const STATE_TIMELINE_FILE_NAME: &str = "spanfold-state-timeline.csv";
const NAB_DATASET_ID: &str = "nab";
const SAMPLE_INDEX_COLUMN: &str = "sample_index";
const COMPARISON_STATE_COLUMN: &str = "comparison_state";
const DISPLAY_NAME: &str = "NAB SpanFold comparison state by sample";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum ComparisonState {
    MissedAnomaly,
    DetectedAnomaly,
    FalsePositive,
    OutsideAnomaly,
}

#[derive(Debug, Serialize)]
struct StateTimelineRow<'a> {
    sample_index: usize,
    comparison_state: ComparisonState,
    timestamp: &'a str,
    value: f64,
    spanfold_row_ids: String,
}

pub(super) fn prepare_state_timeline(
    analysis_directory: &Path,
    samples: &[NabSample],
    intervals: &SpanfoldIntervalDataset,
) -> Result<()> {
    let timeline_path = analysis_directory.join(STATE_TIMELINE_FILE_NAME);
    let mut rows = samples
        .iter()
        .enumerate()
        .map(|(sample_index, sample)| state_timeline_row(sample_index, sample, intervals.rows()))
        .collect::<Result<Vec<_>>>()?;
    rows.sort_by_key(|row| (row.comparison_state, row.sample_index));

    let file = fs::File::create(&timeline_path).map_err(|source| {
        ShowcaseError::workflow(
            NAB_DATASET_ID,
            "create the comparison-state timeline",
            source,
        )
    })?;
    let mut writer = csv::Writer::from_writer(file);
    for row in rows {
        writer.serialize(row).map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "write a comparison-state sample", source)
        })?;
    }
    writer.flush().map_err(|source| {
        ShowcaseError::workflow(
            NAB_DATASET_ID,
            "flush the comparison-state timeline",
            source,
        )
    })?;

    let session_directory = analysis_directory.join(STATE_TIMELINE_SESSION_DIRECTORY_NAME);
    if session_directory.exists() {
        fs::remove_dir_all(&session_directory).map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "replace the comparison-state timeline session",
                source,
            )
        })?;
    }
    LocalCsvSession::timeline(timeline_path, SAMPLE_INDEX_COLUMN, COMPARISON_STATE_COLUMN)
        .display_name(DISPLAY_NAME)
        .evidence_key(SAMPLE_INDEX_COLUMN)
        .prepare(session_directory)
        .map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "prepare the comparison-state timeline session",
                source,
            )
        })?;
    Ok(())
}

fn state_timeline_row<'a>(
    sample_index: usize,
    sample: &'a NabSample,
    intervals: &[SpanfoldIntervalRow],
) -> Result<StateTimelineRow<'a>> {
    let position = sample_index as i64;
    let matching = intervals
        .iter()
        .filter(|row| {
            matches!(
                row.row_family,
                SpanfoldIntervalFamily::Overlap
                    | SpanfoldIntervalFamily::Residual
                    | SpanfoldIntervalFamily::Missing
            ) && row.start <= position
                && position < row.end
        })
        .collect::<Vec<_>>();
    let state = comparison_state(&matching).map_err(|reason| ShowcaseError::InvalidArtifact {
        dataset_id: NAB_DATASET_ID,
        path: Path::new(STATE_TIMELINE_FILE_NAME).to_path_buf(),
        reason: format!("sample {sample_index} {reason}"),
    })?;
    let spanfold_row_ids = serde_json::to_string(
        &matching
            .iter()
            .map(|row| row.spanfold_row_id.as_str())
            .collect::<Vec<_>>(),
    )
    .map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "encode SpanFold timeline evidence", source)
    })?;
    Ok(StateTimelineRow {
        sample_index,
        comparison_state: state,
        timestamp: &sample.timestamp,
        value: sample.value,
        spanfold_row_ids,
    })
}

fn comparison_state(rows: &[&SpanfoldIntervalRow]) -> std::result::Result<ComparisonState, String> {
    let mut families = rows.iter().map(|row| row.row_family);
    let Some(family) = families.next() else {
        return Ok(ComparisonState::OutsideAnomaly);
    };
    if families.any(|candidate| candidate != family) {
        return Err("belongs to conflicting SpanFold comparison families".to_string());
    }
    match family {
        SpanfoldIntervalFamily::Overlap => Ok(ComparisonState::DetectedAnomaly),
        SpanfoldIntervalFamily::Residual => Ok(ComparisonState::MissedAnomaly),
        SpanfoldIntervalFamily::Missing => Ok(ComparisonState::FalsePositive),
        _ => Err("belongs to a non-state SpanFold comparison family".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use rawscope::spanfold::SpanfoldIntervalTransform;

    use super::prepare_state_timeline;
    use crate::analyse::{compare_windows, inputs::NabSample, SampleRange};

    #[test]
    fn timeline_classifies_every_sample_from_spanfold_ranges_and_preserves_source_values() {
        let comparison = compare_windows(
            &[SampleRange { start: 1, end: 4 }],
            &[
                SampleRange { start: 2, end: 3 },
                SampleRange { start: 5, end: 6 },
            ],
        )
        .unwrap();
        let intervals = SpanfoldIntervalTransform::core_comparison()
            .transform(&comparison)
            .unwrap();
        let samples = (0..7)
            .map(|index| NabSample {
                timestamp: format!("t-{index}"),
                value: index as f64 * 10.0,
            })
            .collect::<Vec<_>>();
        let fixture = Fixture::new();

        prepare_state_timeline(&fixture.root, &samples, &intervals).unwrap();

        let mut reader =
            csv::Reader::from_path(fixture.root.join(super::STATE_TIMELINE_FILE_NAME)).unwrap();
        let rows = reader.records().map(|row| row.unwrap()).collect::<Vec<_>>();
        assert_eq!(rows.len(), samples.len());
        assert!(rows.iter().any(|row| {
            row.get(0) == Some("2")
                && row.get(1) == Some("detected_anomaly")
                && row.get(2) == Some("t-2")
                && row.get(3) == Some("20.0")
        }));
        assert!(rows
            .iter()
            .any(|row| row.get(0) == Some("1") && row.get(1) == Some("missed_anomaly")));
        assert!(rows
            .iter()
            .any(|row| row.get(0) == Some("5") && row.get(1) == Some("false_positive")));
        assert!(rows
            .iter()
            .any(|row| row.get(0) == Some("0") && row.get(1) == Some("outside_anomaly")));
    }

    struct Fixture {
        root: std::path::PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rawscope-nab-state-timeline-{}",
                std::process::id()
            ));
            if root.exists() {
                fs::remove_dir_all(&root).unwrap();
            }
            fs::create_dir(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    use std::fs;
}
