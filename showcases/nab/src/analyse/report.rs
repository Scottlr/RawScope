//! Family-level duration and aggregate coverage reporting.

use std::{collections::BTreeMap, fs, path::Path};

use rawscope::spanfold::SpanfoldIntervalDataset;
use rawscope_showcase_support::{Result, ShowcaseError};
use serde::Serialize;
use spanfold::ComparisonResult;

const NAB_DATASET_ID: &str = "nab";

#[derive(Debug, Serialize)]
struct AggregateRow {
    row_family: String,
    interval_count: usize,
    total_duration_samples: i64,
    mean_duration_samples: f64,
    overall_target_samples: Option<i128>,
    overall_covered_samples: Option<i128>,
    overall_coverage_ratio: Option<f64>,
}

pub(super) fn write_aggregations(
    path: &Path,
    dataset: &SpanfoldIntervalDataset,
    comparison: &ComparisonResult,
) -> Result<()> {
    let coverage = comparison.coverage_summaries.first();
    let mut grouped = BTreeMap::<String, Vec<i64>>::new();
    for row in dataset.rows() {
        grouped
            .entry(row.row_family.as_str().to_string())
            .or_default()
            .push(row.duration);
    }
    let file = fs::File::create(path).map_err(|source| {
        ShowcaseError::workflow(
            NAB_DATASET_ID,
            "create the SpanFold aggregation table",
            source,
        )
    })?;
    let mut writer = csv::Writer::from_writer(file);
    for (row_family, durations) in grouped {
        let total_duration_samples = durations.iter().sum::<i64>();
        writer
            .serialize(AggregateRow {
                row_family,
                interval_count: durations.len(),
                total_duration_samples,
                mean_duration_samples: total_duration_samples as f64 / durations.len() as f64,
                overall_target_samples: coverage.map(|summary| summary.target_magnitude_exact),
                overall_covered_samples: coverage.map(|summary| summary.covered_magnitude_exact),
                overall_coverage_ratio: coverage.map(|summary| summary.coverage_ratio),
            })
            .map_err(|source| {
                ShowcaseError::workflow(NAB_DATASET_ID, "write a SpanFold aggregation row", source)
            })?;
    }
    writer.flush().map_err(|source| {
        ShowcaseError::workflow(
            NAB_DATASET_ID,
            "flush the SpanFold aggregation table",
            source,
        )
    })
}
