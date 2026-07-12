//! Bounded aggregate context retained in v3 evidence artifacts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateEvidenceBin {
    pub bin_x: u32,
    pub bin_y: u32,
    pub count: u32,
    pub row_id_sample: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterAggregateEvidenceContext {
    pub bin_limit: usize,
    pub bins: Vec<AggregateEvidenceBin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineAggregateEvidenceContext {
    pub bin_limit: usize,
    pub bins: Vec<AggregateEvidenceBin>,
}
