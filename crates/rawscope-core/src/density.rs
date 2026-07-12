//! Checked density grid types for CPU and GPU-facing paths.

use std::{error::Error, fmt, num::NonZeroU32, ops::Deref};

use crate::RowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSizeError {
    ZeroWidth,
    ZeroHeight,
    BinCountOverflow { width: u32, height: u32 },
    CountLengthMismatch { expected: usize, actual: usize },
    CoordinateOutOfBounds { x: u32, y: u32 },
}

impl fmt::Display for GridSizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => write!(formatter, "grid width must be positive"),
            Self::ZeroHeight => write!(formatter, "grid height must be positive"),
            Self::BinCountOverflow { width, height } => {
                write!(
                    formatter,
                    "grid {width}x{height} exceeds addressable bin count"
                )
            }
            Self::CountLengthMismatch { expected, actual } => {
                write!(
                    formatter,
                    "grid count length {actual} does not match {expected}"
                )
            }
            Self::CoordinateOutOfBounds { x, y } => {
                write!(formatter, "grid coordinate ({x}, {y}) is out of bounds")
            }
        }
    }
}

impl Error for GridSizeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSizeFields {
    pub width: u32,
    pub height: u32,
}

/// The positive dimensions of a density grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    fields: GridSizeFields,
    bin_count: usize,
}

impl GridSize {
    pub fn try_new(width: u32, height: u32) -> Result<Self, GridSizeError> {
        let width = NonZeroU32::new(width).ok_or(GridSizeError::ZeroWidth)?;
        let height = NonZeroU32::new(height).ok_or(GridSizeError::ZeroHeight)?;
        let bin_count = (width.get() as usize)
            .checked_mul(height.get() as usize)
            .ok_or(GridSizeError::BinCountOverflow {
                width: width.get(),
                height: height.get(),
            })?;
        Ok(Self {
            fields: GridSizeFields {
                width: width.get(),
                height: height.get(),
            },
            bin_count,
        })
    }

    pub fn new(width: u32, height: u32) -> Self {
        Self::try_new(width, height).expect("validated density grid size")
    }

    pub fn width(self) -> u32 {
        self.fields.width
    }

    pub fn height(self) -> u32 {
        self.fields.height
    }

    pub fn bin_count(self) -> usize {
        self.bin_count
    }
}

impl Deref for GridSize {
    type Target = GridSizeFields;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

/// A bin count that cannot be zero and cannot overflow a `u64` count boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BinCount(NonZeroU32);

impl BinCount {
    pub fn try_new(value: u32) -> Result<Self, GridSizeError> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(GridSizeError::ZeroWidth)
    }

    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// One bin in a density grid, including the contributing row ids.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DensityBin {
    pub row_count: u32,
    row_ids: Vec<RowId>,
}

impl DensityBin {
    /// Records one row in the bin with checked count accumulation.
    pub fn push(&mut self, row_id: RowId) -> Result<(), DensityCountError> {
        self.row_count = self
            .row_count
            .checked_add(1)
            .ok_or(DensityCountError::Overflow)?;
        self.row_ids.push(row_id);
        Ok(())
    }

    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityCountError {
    Overflow,
}

impl fmt::Display for DensityCountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "density bin row count overflowed u64")
    }
}

impl Error for DensityCountError {}

/// A simple row-count density grid used by CPU reference renderers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DensityGrid {
    size: GridSize,
    bins: Vec<DensityBin>,
}

impl DensityGrid {
    pub fn new(size: GridSize) -> Self {
        Self {
            size,
            bins: vec![DensityBin::default(); size.bin_count()],
        }
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    pub fn bins(&self) -> &[DensityBin] {
        &self.bins
    }

    pub fn bin(&self, x: u32, y: u32) -> &DensityBin {
        let index = self.bin_index(x, y).expect("validated density coordinate");
        &self.bins[index]
    }

    pub fn bin_mut(&mut self, x: u32, y: u32) -> &mut DensityBin {
        let index = self.bin_index(x, y).expect("validated density coordinate");
        &mut self.bins[index]
    }

    pub fn try_bin(&self, x: u32, y: u32) -> Result<&DensityBin, GridSizeError> {
        let index = self.bin_index(x, y)?;
        Ok(&self.bins[index])
    }

    pub fn total_row_count(&self) -> u64 {
        self.bins.iter().map(|bin| bin.row_count as u64).sum()
    }

    fn bin_index(&self, x: u32, y: u32) -> Result<usize, GridSizeError> {
        if x >= self.size.width() || y >= self.size.height() {
            return Err(GridSizeError::CoordinateOutOfBounds { x, y });
        }
        Ok((y as usize) * (self.size.width() as usize) + (x as usize))
    }
}

/// A flattened count-only density grid used by GPU readback paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DensityCountGrid {
    size: GridSize,
    counts: Vec<u32>,
}

impl DensityCountGrid {
    pub fn try_new(size: GridSize, counts: Vec<u32>) -> Result<Self, GridSizeError> {
        if counts.len() != size.bin_count() {
            return Err(GridSizeError::CountLengthMismatch {
                expected: size.bin_count(),
                actual: counts.len(),
            });
        }
        Ok(Self { size, counts })
    }

    pub fn new(size: GridSize, counts: Vec<u32>) -> Self {
        Self::try_new(size, counts).expect("validated density count grid")
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    pub fn width(&self) -> u32 {
        self.size.width()
    }

    pub fn height(&self) -> u32 {
        self.size.height()
    }

    pub fn counts(&self) -> &[u32] {
        &self.counts
    }

    pub fn count(&self, x: u32, y: u32) -> u32 {
        let index = (y as usize) * (self.size.width() as usize) + (x as usize);
        self.counts[index]
    }

    pub fn total_count(&self) -> u64 {
        self.counts.iter().map(|count| *count as u64).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_grid_rejects_zero_dimensions_and_mismatched_counts() {
        assert_eq!(GridSize::try_new(0, 2), Err(GridSizeError::ZeroWidth));
        let size = GridSize::try_new(3, 2).unwrap();
        assert!(matches!(
            DensityCountGrid::try_new(size, vec![1]),
            Err(GridSizeError::CountLengthMismatch { .. })
        ));
    }

    #[test]
    fn count_grid_indexes_flattened_counts_by_coordinate() {
        let size = GridSize::try_new(3, 2).unwrap();
        let grid = DensityCountGrid::try_new(size, vec![1, 2, 3, 4, 5, 6]).unwrap();

        assert_eq!(grid.width(), 3);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.count(0, 0), 1);
        assert_eq!(grid.count(2, 1), 6);
        assert_eq!(grid.total_count(), 21);
    }

    #[test]
    fn density_bin_count_is_checked() {
        let mut bin = DensityBin {
            row_count: u32::MAX,
            ..DensityBin::default()
        };
        assert_eq!(bin.push(RowId(1)), Err(DensityCountError::Overflow));
    }
}
