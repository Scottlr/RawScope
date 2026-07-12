//! Shared foundational types for RawScope.

mod column;
mod density;
mod generation;
mod range;
mod row;
mod selection;

pub use column::{ColumnId, PositiveRowLimit, PositiveRowLimitError};
pub use density::{
    BinCount, DensityBin, DensityCountError, DensityCountGrid, DensityGrid, GridSize, GridSizeError,
};
pub use generation::{Generation, GenerationCounter};
pub use range::{
    F32Domain, F32Range, F32RangeFields, ObservedF32Extent, ObservedU64Extent, RangeError,
    U64Range, U64RangeFields,
};
pub use row::RowId;
pub use selection::{
    CoreLaneRange, CoreLaneRangeError, CoreLaneRangeFields, SelectionId, ViewId, VisualSelection,
    VisualSelectionError, VisualSelectionGeometry, VisualSelectionKind,
};
