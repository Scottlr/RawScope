//! Render-owned aggregate selection context construction for timeline evidence v3.

use std::collections::HashSet;

use rawscope_core::RowId;
use rawscope_evidence::{AggregateEvidenceBin, TimelineAggregateEvidenceContext};

use crate::{AggregateBinSample, TimelineAggregateOverview};

const MAX_AGGREGATE_CONTEXT_BINS: usize = 16;

/// Builds bounded timeline aggregate context for evidence v3.
pub fn timeline_aggregate_evidence_context(
    overview: &TimelineAggregateOverview,
    selected_row_ids: &[RowId],
) -> TimelineAggregateEvidenceContext {
    let selected_row_id_set = selected_row_ids
        .iter()
        .map(|row_id| row_id.0)
        .collect::<HashSet<_>>();
    let mut selected_bins = overview
        .bins
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| {
            let intersects_selected_sample = bin
                .row_ids
                .iter()
                .any(|row_id| selected_row_id_set.contains(&row_id.0));
            intersects_selected_sample.then(|| candidate_bin(index, overview.grid_width, bin))
        })
        .collect::<Vec<_>>();
    selected_bins.sort_by(|left, right| {
        left.bin_y
            .cmp(&right.bin_y)
            .then_with(|| left.bin_x.cmp(&right.bin_x))
    });
    selected_bins.truncate(MAX_AGGREGATE_CONTEXT_BINS);

    let selected_bin_indices = selected_bins
        .iter()
        .map(|candidate| candidate.index)
        .collect::<HashSet<_>>();
    let remaining_capacity = MAX_AGGREGATE_CONTEXT_BINS.saturating_sub(selected_bins.len());

    let mut densest_bins = overview
        .bins
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| {
            let bin_has_context = bin.count > 0 && !selected_bin_indices.contains(&index);
            bin_has_context.then(|| candidate_bin(index, overview.grid_width, bin))
        })
        .collect::<Vec<_>>();
    densest_bins.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.bin_y.cmp(&right.bin_y))
            .then_with(|| left.bin_x.cmp(&right.bin_x))
    });
    densest_bins.truncate(remaining_capacity);

    let bins = selected_bins
        .into_iter()
        .chain(densest_bins)
        .map(|candidate| AggregateEvidenceBin {
            bin_x: candidate.bin_x,
            bin_y: candidate.bin_y,
            count: candidate.count,
            row_id_sample: candidate.row_ids.iter().map(|row_id| row_id.0).collect(),
        })
        .collect();

    TimelineAggregateEvidenceContext {
        bin_limit: MAX_AGGREGATE_CONTEXT_BINS,
        bins,
    }
}

#[derive(Debug, Clone, Copy)]
struct CandidateBin<'a> {
    index: usize,
    bin_x: u32,
    bin_y: u32,
    count: u32,
    row_ids: &'a [RowId],
}

fn candidate_bin<'a>(
    index: usize,
    grid_width: u32,
    bin: &'a AggregateBinSample,
) -> CandidateBin<'a> {
    CandidateBin {
        index,
        bin_x: index as u32 % grid_width,
        bin_y: index as u32 / grid_width,
        count: bin.count,
        row_ids: &bin.row_ids,
    }
}
