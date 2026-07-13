//! Pinned NAB window construction, SpanFold comparison, and session preparation.

mod inputs;
mod report;
mod state_timeline;

use std::{fs, path::Path};

use rawscope::spanfold::SpanfoldIntervalTransform;
use rawscope_showcase_support::{Result, ShowcaseContext, ShowcaseError};
use spanfold::{for_events, ComparisonNormalizationPolicy, ComparisonResult};

use self::inputs::{read_inputs, SampleRange};

pub const DURATION_SESSION_DIRECTORY_NAME: &str = "spanfold-duration-session";
pub const TIMELINE_SESSION_DIRECTORY_NAME: &str = "spanfold-timeline-session";
pub const STATE_TIMELINE_SESSION_DIRECTORY_NAME: &str =
    state_timeline::STATE_TIMELINE_SESSION_DIRECTORY_NAME;
pub const AGGREGATIONS_FILE_NAME: &str = "spanfold-aggregations.csv";

const NAB_DATASET_ID: &str = "nab";
const NAB_WINDOW_NAME: &str = "Anomaly";
const NAB_WINDOW_KEY: &str = "speed_7578";
const GROUND_TRUTH_SOURCE: &str = "nab-ground-truth";
const DETECTOR_SOURCE: &str = "numenta-standard";
const COMPARISON_NAME: &str = "NAB Numenta standard threshold vs ground truth";

#[derive(Debug)]
struct WindowEvent {
    sample_index: i64,
    source: &'static str,
    active: bool,
}

pub fn analyse(context: &ShowcaseContext) -> Result<()> {
    let inputs = read_inputs(&context.paths.downloads)?;
    let comparison = compare_windows(&inputs.ground_truth, &inputs.detector)?;
    let transformed = SpanfoldIntervalTransform::core_comparison()
        .transform(&comparison)
        .map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "adapt the SpanFold comparison", source)
        })?;

    fs::create_dir_all(&context.paths.analysis).map_err(|source| {
        ShowcaseError::workflow(NAB_DATASET_ID, "create the analysis directory", source)
    })?;
    let duration_session = context.paths.analysis.join(DURATION_SESSION_DIRECTORY_NAME);
    let timeline_session = context.paths.analysis.join(TIMELINE_SESSION_DIRECTORY_NAME);
    replace_directory(&duration_session)?;
    replace_directory(&timeline_session)?;
    transformed
        .clone()
        .prepare_session(&duration_session)
        .map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "prepare the SpanFold duration session",
                source,
            )
        })?;
    transformed
        .clone()
        .prepare_timeline_session(&timeline_session)
        .map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "prepare the SpanFold timeline session",
                source,
            )
        })?;
    state_timeline::prepare_state_timeline(&context.paths.analysis, &inputs.samples, &transformed)?;
    report::write_aggregations(
        &context.paths.analysis.join(AGGREGATIONS_FILE_NAME),
        &transformed,
        &comparison,
    )
}

fn compare_windows(target: &[SampleRange], against: &[SampleRange]) -> Result<ComparisonResult> {
    let mut events = window_events(target, GROUND_TRUTH_SOURCE);
    events.extend(window_events(against, DETECTOR_SOURCE));
    events.sort_by_key(|event| (event.sample_index, event.active, event.source));

    let mut pipeline = for_events::<WindowEvent>()
        .record_windows()
        .with_event_time(|event| event.sample_index)
        .track_window(NAB_WINDOW_NAME, |_| NAB_WINDOW_KEY, |event| event.active)
        .build()
        .map_err(|source| {
            ShowcaseError::workflow(NAB_DATASET_ID, "build the SpanFold event pipeline", source)
        })?;
    for event in events {
        let source_name = event.source;
        pipeline
            .ingest(event, Some(source_name), None)
            .map_err(|source| {
                ShowcaseError::workflow(NAB_DATASET_ID, "ingest NAB window boundaries", source)
            })?;
    }

    Ok(pipeline
        .history()
        .compare(COMPARISON_NAME)
        .target_source(GROUND_TRUTH_SOURCE)
        .against_source(DETECTOR_SOURCE)
        .scope_window(NAB_WINDOW_NAME)
        .normalization(ComparisonNormalizationPolicy::event_time())
        .overlap()
        .residual()
        .missing()
        .coverage()
        .run())
}

fn window_events(ranges: &[SampleRange], source: &'static str) -> Vec<WindowEvent> {
    ranges
        .iter()
        .flat_map(|range| {
            [
                WindowEvent {
                    sample_index: range.start,
                    source,
                    active: true,
                },
                WindowEvent {
                    sample_index: range.end,
                    source,
                    active: false,
                },
            ]
        })
        .collect()
}

fn replace_directory(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|source| {
            ShowcaseError::workflow(
                NAB_DATASET_ID,
                "replace a prepared analysis session",
                source,
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rawscope::spanfold::{SpanfoldIntervalFamily, SpanfoldIntervalTransform};

    use super::{compare_windows, SampleRange};

    #[test]
    fn spanfold_comparison_preserves_overlap_residual_and_false_positive_ranges() {
        let comparison = compare_windows(
            &[SampleRange { start: 10, end: 20 }],
            &[
                SampleRange { start: 12, end: 14 },
                SampleRange { start: 30, end: 31 },
            ],
        )
        .unwrap();
        let dataset = SpanfoldIntervalTransform::core_comparison()
            .transform(&comparison)
            .unwrap();
        let family_ranges = dataset
            .rows()
            .iter()
            .map(|row| (row.row_family, row.start, row.end))
            .collect::<Vec<_>>();

        assert!(family_ranges.contains(&(SpanfoldIntervalFamily::Overlap, 12, 14)));
        assert!(family_ranges.contains(&(SpanfoldIntervalFamily::Residual, 10, 12)));
        assert!(family_ranges.contains(&(SpanfoldIntervalFamily::Residual, 14, 20)));
        assert!(family_ranges.contains(&(SpanfoldIntervalFamily::Missing, 30, 31)));
    }
}
