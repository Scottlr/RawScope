//! Generic visual records shared by synthetic and local datasets.

use rawscope_core::RowId;

use crate::{SyntheticEventType, SyntheticPointCategory};

/// Optional synthetic annotation for a scatter point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterPointKind {
    Synthetic(SyntheticPointCategory),
    Unclassified,
}

impl ScatterPointKind {
    /// Returns the synthetic category when this point came from a synthetic generator.
    pub fn synthetic_category(self) -> Option<SyntheticPointCategory> {
        match self {
            Self::Synthetic(category) => Some(category),
            Self::Unclassified => None,
        }
    }
}

/// One visual scatter point record.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterPointRecord {
    pub row_id: RowId,
    pub x: f32,
    pub y: f32,
    pub kind: ScatterPointKind,
}

/// Optional synthetic annotation for a timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    Synthetic(SyntheticEventType),
    Unclassified,
}

impl TimelineEventKind {
    /// Returns the synthetic event type when this event came from a synthetic generator.
    pub fn synthetic_event_type(self) -> Option<SyntheticEventType> {
        match self {
            Self::Synthetic(event_type) => Some(event_type),
            Self::Unclassified => None,
        }
    }
}

/// One visual timeline event record.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEventRecord {
    pub row_id: RowId,
    pub timestamp: u64,
    pub lane: u32,
    pub value: f32,
    pub kind: TimelineEventKind,
}
