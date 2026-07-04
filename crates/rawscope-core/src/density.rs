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
        bin_index(self.size, x, y)
    }
}

/// A flattened count-only density grid used by GPU readback paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DensityCountGrid {
    size: GridSize,
    counts: Vec<u32>,
}

impl DensityCountGrid {
    /// Creates a flattened count grid with the given dimensions.
    pub fn new(size: GridSize, counts: Vec<u32>) -> Self {
        assert_eq!(
            counts.len(),
            size.bin_count(),
            "count length must match grid dimensions"
        );

        Self { size, counts }
    }

    /// Returns the size of the grid.
    pub fn size(&self) -> GridSize {
        self.size
    }

    /// Returns the grid width in bins.
    pub fn width(&self) -> u32 {
        self.size.width
    }

    /// Returns the grid height in bins.
    pub fn height(&self) -> u32 {
        self.size.height
    }

    /// Returns the flattened row-count bins.
    pub fn counts(&self) -> &[u32] {
        &self.counts
    }

    /// Returns one bin count by x/y coordinate.
    pub fn count(&self, x: u32, y: u32) -> u32 {
        let index = bin_index(self.size, x, y);
        self.counts[index]
    }

    /// Returns the total number of binned rows.
    pub fn total_count(&self) -> u64 {
        self.counts.iter().map(|count| *count as u64).sum()
    }
}

fn bin_index(size: GridSize, x: u32, y: u32) -> usize {
    assert!(x < size.width, "x bin out of range");
    assert!(y < size.height, "y bin out of range");
    (y as usize) * (size.width as usize) + (x as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_grid_indexes_flattened_counts_by_coordinate() {
        let grid = DensityCountGrid::new(GridSize::new(3, 2), vec![1, 2, 3, 4, 5, 6]);

        assert_eq!(grid.width(), 3);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.count(0, 0), 1);
        assert_eq!(grid.count(2, 1), 6);
        assert_eq!(grid.total_count(), 21);
    }
}
