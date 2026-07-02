//! Shared foundational types for RawScope.

mod density;
mod range;
mod row;
mod selection;

pub use density::{DensityBin, DensityGrid, GridSize};
pub use range::{F32Range, U64Range};
pub use row::RowId;
pub use selection::SelectionId;
