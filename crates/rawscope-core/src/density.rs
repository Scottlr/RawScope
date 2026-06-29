//! Shared density grid types for CPU and future GPU render paths.

use crate::RowId;

/// The size of a density grid in bins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub width: u32,
    pub height: u32,
}

impl GridSize {
    /// Creates a grid size for a non-empty density surface.
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0, "grid width must be positive");
        assert!(height > 0, "grid height must be positive");
        Self { width, height }
    }

    /// Returns the total number of bins in the grid.
    pub fn bin_count(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}

/// One bin in a density grid, including the contributing row ids.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DensityBin {
    pub row_count: u32,
    row_ids: Vec<RowId>,
}

impl DensityBin {
    /// Records one row in the bin.
    pub fn push(&mut self, row_id: RowId) {
        self.row_count += 1;
        self.row_ids.push(row_id);
    }

    /// Returns the contributing row ids for this bin.
    pub fn row_ids(&self) -> &[RowId] {
        &self.row_ids
    }
}

/// A simple row-count density grid used by CPU reference renderers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DensityGrid {
    size: GridSize,
    bins: Vec<DensityBin>,
}

impl DensityGrid {
    /// Creates an empty grid with the given size.
    pub fn new(size: GridSize) -> Self {
        Self {
            size,
            bins: vec![DensityBin::default(); size.bin_count()],
        }
    }

    /// Returns the size of the grid.
    pub fn size(&self) -> GridSize {
        self.size
    }

    /// Returns the flattened bins.
    pub fn bins(&self) -> &[DensityBin] {
        &self.bins
    }

    /// Returns the bin at the given coordinates.
    pub fn bin(&self, x: u32, y: u32) -> &DensityBin {
        let index = self.bin_index(x, y);
        &self.bins[index]
    }

    /// Returns the mutable bin at the given coordinates.
    pub fn bin_mut(&mut self, x: u32, y: u32) -> &mut DensityBin {
        let index = self.bin_index(x, y);
        &mut self.bins[index]
    }

    /// Returns the total number of rows recorded in the grid.
    pub fn total_row_count(&self) -> u64 {
        self.bins.iter().map(|bin| bin.row_count as u64).sum()
    }

    fn bin_index(&self, x: u32, y: u32) -> usize {
        assert!(x < self.size.width, "x bin out of range");
        assert!(y < self.size.height, "y bin out of range");
        (y as usize) * (self.size.width as usize) + (x as usize)
    }
}
