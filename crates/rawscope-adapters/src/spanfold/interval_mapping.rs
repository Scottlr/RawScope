//! Internal mapping from SpanFold result families to rectangular adapter rows.

use spanfold::{
    ComparisonResult, ComparisonRowFinality, ComparisonSide, ContainmentStatus, RowRange,
    TemporalAxis,
};

use super::{
    error::SpanfoldAdapterError,
    interval::{SpanfoldIntervalDataset, SpanfoldIntervalFamily, SpanfoldIntervalRow},
};

pub(super) fn transform_comparison(
    result: &ComparisonResult,
    families: &[SpanfoldIntervalFamily],
) -> Result<SpanfoldIntervalDataset, SpanfoldAdapterError> {
    if !result.is_valid {
        let diagnostics = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect();
        return Err(SpanfoldAdapterError::InvalidComparison { diagnostics });
    }

    let pending_rows = collect_pending_rows(result, families)?;
    let first_row = pending_rows
        .first()
        .ok_or(SpanfoldAdapterError::NoSelectedIntervals)?;
    let temporal_axis = first_row.range.axis;
    let axis_label = temporal_axis_label(temporal_axis)?;
    let clock = first_row.range.clock.clone();

    for row in pending_rows.iter().skip(1) {
        let actual_axis_label = temporal_axis_label(row.range.axis)?;
        let same_domain = row.range.axis == temporal_axis && row.range.clock == clock;
        if !same_domain {
            return Err(SpanfoldAdapterError::MixedTemporalDomain {
                expected_axis: axis_label,
                expected_clock: clock.clone(),
                actual_axis: actual_axis_label,
                actual_clock: row.range.clock.clone(),
            });
        }
    }

    let origin = pending_rows
        .iter()
        .map(|row| row.range.start)
        .min()
        .ok_or(SpanfoldAdapterError::NoSelectedIntervals)?;
    let rows = pending_rows
        .into_iter()
        .map(|row| materialize_row(row, &result.plan_name, axis_label, origin))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SpanfoldIntervalDataset::new(
        result.plan_name.clone(),
        temporal_axis,
        clock,
        origin,
        rows,
    ))
}

#[derive(Debug)]
struct PendingIntervalRow<'a> {
    family: SpanfoldIntervalFamily,
    metadata: &'a ComparisonRowFinality,
    window_name: &'a str,
    key: &'a str,
    partition: Option<&'a str>,
    range: &'a RowRange,
    target_magnitude: Option<i64>,
    covered_magnitude: Option<i64>,
    side: Option<&'static str>,
    status: Option<&'static str>,
    target_record_ids: &'a [String],
    against_record_ids: &'a [String],
}

fn collect_pending_rows<'a>(
    result: &'a ComparisonResult,
    families: &[SpanfoldIntervalFamily],
) -> Result<Vec<PendingIntervalRow<'a>>, SpanfoldAdapterError> {
    let mut rows = Vec::new();
    for family in families {
        match family {
            SpanfoldIntervalFamily::Overlap => {
                rows.extend(result.overlap_rows_with_finality()?.map(|entry| {
                    let row = entry.row;
                    PendingIntervalRow {
                        family: *family,
                        metadata: entry.metadata,
                        window_name: &row.window_name,
                        key: &row.key,
                        partition: row.partition.as_deref(),
                        range: &row.range,
                        target_magnitude: None,
                        covered_magnitude: None,
                        side: None,
                        status: None,
                        target_record_ids: &row.target_record_ids,
                        against_record_ids: &row.against_record_ids,
                    }
                }));
            }
            SpanfoldIntervalFamily::Residual => {
                rows.extend(result.residual_rows_with_finality()?.map(|entry| {
                    let row = entry.row;
                    PendingIntervalRow {
                        family: *family,
                        metadata: entry.metadata,
                        window_name: &row.window_name,
                        key: &row.key,
                        partition: row.partition.as_deref(),
                        range: &row.range,
                        target_magnitude: None,
                        covered_magnitude: None,
                        side: None,
                        status: None,
                        target_record_ids: &row.target_record_ids,
                        against_record_ids: &[],
                    }
                }));
            }
            SpanfoldIntervalFamily::Missing => {
                rows.extend(result.missing_rows_with_finality()?.map(|entry| {
                    let row = entry.row;
                    PendingIntervalRow {
                        family: *family,
                        metadata: entry.metadata,
                        window_name: &row.window_name,
                        key: &row.key,
                        partition: row.partition.as_deref(),
                        range: &row.range,
                        target_magnitude: None,
                        covered_magnitude: None,
                        side: None,
                        status: None,
                        target_record_ids: &[],
                        against_record_ids: &row.against_record_ids,
                    }
                }));
            }
            SpanfoldIntervalFamily::Coverage => {
                rows.extend(result.coverage_rows_with_finality()?.map(|entry| {
                    let row = entry.row;
                    PendingIntervalRow {
                        family: *family,
                        metadata: entry.metadata,
                        window_name: &row.window_name,
                        key: &row.key,
                        partition: row.partition.as_deref(),
                        range: &row.range,
                        target_magnitude: Some(row.target_magnitude),
                        covered_magnitude: Some(row.covered_magnitude),
                        side: None,
                        status: None,
                        target_record_ids: &row.target_record_ids,
                        against_record_ids: &row.against_record_ids,
                    }
                }));
            }
            SpanfoldIntervalFamily::Gap => {
                rows.extend(result.gap_rows_with_finality()?.map(|entry| {
                    let row = entry.row;
                    PendingIntervalRow {
                        family: *family,
                        metadata: entry.metadata,
                        window_name: &row.window_name,
                        key: &row.key,
                        partition: row.partition.as_deref(),
                        range: &row.range,
                        target_magnitude: None,
                        covered_magnitude: None,
                        side: None,
                        status: None,
                        target_record_ids: &[],
                        against_record_ids: &[],
                    }
                }));
            }
            SpanfoldIntervalFamily::SymmetricDifference => {
                rows.extend(
                    result
                        .symmetric_difference_rows_with_finality()?
                        .map(|entry| {
                            let row = entry.row;
                            PendingIntervalRow {
                                family: *family,
                                metadata: entry.metadata,
                                window_name: &row.window_name,
                                key: &row.key,
                                partition: row.partition.as_deref(),
                                range: &row.range,
                                target_magnitude: None,
                                covered_magnitude: None,
                                side: Some(comparison_side_label(&row.side)),
                                status: None,
                                target_record_ids: &row.target_record_ids,
                                against_record_ids: &row.against_record_ids,
                            }
                        }),
                );
            }
            SpanfoldIntervalFamily::Containment => {
                rows.extend(result.containment_rows_with_finality()?.map(|entry| {
                    let row = entry.row;
                    PendingIntervalRow {
                        family: *family,
                        metadata: entry.metadata,
                        window_name: &row.window_name,
                        key: &row.key,
                        partition: row.partition.as_deref(),
                        range: &row.range,
                        target_magnitude: None,
                        covered_magnitude: None,
                        side: None,
                        status: Some(containment_status_label(&row.status)),
                        target_record_ids: &row.target_record_ids,
                        against_record_ids: &row.container_record_ids,
                    }
                }));
            }
        }
    }
    Ok(rows)
}

fn materialize_row(
    row: PendingIntervalRow<'_>,
    plan_name: &str,
    temporal_axis: &'static str,
    origin: i64,
) -> Result<SpanfoldIntervalRow, SpanfoldAdapterError> {
    let duration =
        row.range
            .end
            .checked_sub(row.range.start)
            .ok_or(SpanfoldAdapterError::InvalidRange {
                family: row.family.as_str(),
                start: row.range.start,
                end: row.range.end,
            })?;
    if duration < 0 {
        return Err(SpanfoldAdapterError::InvalidRange {
            family: row.family.as_str(),
            start: row.range.start,
            end: row.range.end,
        });
    }

    let start_offset = (i128::from(row.range.start) - i128::from(origin)) as f64;
    let target_record_ids = encode_record_ids(row.family, row.target_record_ids)?;
    let against_record_ids = encode_record_ids(row.family, row.against_record_ids)?;
    let segment_coverage_ratio = match (row.target_magnitude, row.covered_magnitude) {
        (Some(target), Some(covered)) if target > 0 => Some(covered as f64 / target as f64),
        _ => None,
    };

    Ok(SpanfoldIntervalRow {
        spanfold_row_id: row.metadata.row_id.clone(),
        spanfold_finality: row.metadata.finality.clone(),
        spanfold_finality_reason: row.metadata.reason.clone(),
        spanfold_row_version: row.metadata.version,
        spanfold_supersedes_row_id: row.metadata.supersedes_row_id.clone(),
        row_family: row.family,
        plan_name: plan_name.to_string(),
        window_name: row.window_name.to_string(),
        key: row.key.to_string(),
        partition: row.partition.map(str::to_string),
        temporal_axis: temporal_axis.to_string(),
        clock: row.range.clock.clone(),
        origin,
        start: row.range.start,
        end: row.range.end,
        start_offset,
        duration,
        target_magnitude: row.target_magnitude,
        covered_magnitude: row.covered_magnitude,
        segment_coverage_ratio,
        side: row.side.map(str::to_string),
        status: row.status.map(str::to_string),
        target_record_ids,
        against_record_ids,
    })
}

fn encode_record_ids(
    family: SpanfoldIntervalFamily,
    record_ids: &[String],
) -> Result<String, SpanfoldAdapterError> {
    serde_json::to_string(record_ids).map_err(|source| SpanfoldAdapterError::RecordIdentifiers {
        family: family.as_str(),
        source,
    })
}

fn temporal_axis_label(axis: TemporalAxis) -> Result<&'static str, SpanfoldAdapterError> {
    match axis {
        TemporalAxis::ProcessingPosition => Ok("processing_position"),
        TemporalAxis::Timestamp => Ok("timestamp"),
        _ => Err(SpanfoldAdapterError::UnsupportedTemporalAxis {
            axis: format!("{axis:?}"),
        }),
    }
}

fn comparison_side_label(side: &ComparisonSide) -> &'static str {
    match side {
        ComparisonSide::Target => "target",
        ComparisonSide::Against => "against",
    }
}

fn containment_status_label(status: &ContainmentStatus) -> &'static str {
    match status {
        ContainmentStatus::Contained => "contained",
        ContainmentStatus::NotContained => "not_contained",
        ContainmentStatus::LeftOverhang => "left_overhang",
        ContainmentStatus::RightOverhang => "right_overhang",
    }
}
