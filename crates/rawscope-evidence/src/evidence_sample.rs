//! Shared deterministic sampling helpers for CPU-side selection evidence.

use rawscope_core::RowId;

pub trait RowIdSample {
    fn row_id(&self) -> RowId;
}

pub fn insert_lowest_row_id_sample<T: RowIdSample>(
    samples: &mut Vec<T>,
    next_sample: T,
    max_sample_size: usize,
) {
    if max_sample_size == 0 {
        return;
    }

    let sample_has_room = samples.len() < max_sample_size;
    if sample_has_room {
        samples.push(next_sample);
        sort_samples_by_row_id(samples);
        return;
    }

    let Some(last_sample) = samples.last() else {
        return;
    };
    let next_sample_belongs_in_sample = next_sample.row_id().0 < last_sample.row_id().0;
    if next_sample_belongs_in_sample {
        samples.pop();
        samples.push(next_sample);
        sort_samples_by_row_id(samples);
    }
}

fn sort_samples_by_row_id<T: RowIdSample>(samples: &mut [T]) {
    samples.sort_by_key(|sample| sample.row_id().0);
}
