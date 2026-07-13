//! Dataset and grid resource replacement for resident scatter density state.

use rawscope_data::FilterRevision;

use super::super::point_pack::VisualFieldPoint;
use super::{
    checked_point_count, dispatch_chunks,
    resources::{bind_groups, filter_mask_buffer, grid_buffers, params_buffers, point_buffer},
    ResidentExactField,
};
use crate::VisualFieldGpuError;

impl ResidentExactField {
    pub fn replace_dataset<T: VisualFieldPoint>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[T],
        dataset_revision: u64,
    ) -> Result<(), VisualFieldGpuError> {
        if self.dataset_revision == dataset_revision {
            return Ok(());
        }
        self.point_count = checked_point_count(points)?;
        self.point_capacity = points.len();
        self.point_buffer = point_buffer(device, queue, points);
        self.filter_mask_buffer = filter_mask_buffer(device, queue, points.len());
        self.filter_revision = FilterRevision::default();
        self.params_buffers = params_buffers(device, dispatch_chunks(self.point_count).len());
        self.compute_bind_groups = bind_groups(
            device,
            &self.compute_bind_group_layout,
            &self.point_buffer,
            &self.filter_mask_buffer,
            &self.params_buffers,
            &self.count_buffers,
            &self.max_count_buffer,
        );
        self.dataset_revision = dataset_revision;
        Ok(())
    }

    pub fn validate_dataset<T: VisualFieldPoint>(
        &self,
        points: &[T],
    ) -> Result<(), VisualFieldGpuError> {
        checked_point_count(points).map(|_| ())
    }

    pub fn update_filter_mask(
        &mut self,
        queue: &wgpu::Queue,
        mask: &[u32],
        revision: FilterRevision,
    ) -> Result<bool, VisualFieldGpuError> {
        self.validate_filter_mask(mask)?;
        if self.filter_revision == revision {
            return Ok(false);
        }
        queue.write_buffer(&self.filter_mask_buffer, 0, bytemuck::cast_slice(mask));
        self.filter_revision = revision;
        Ok(true)
    }

    pub fn validate_filter_mask(&self, mask: &[u32]) -> Result<(), VisualFieldGpuError> {
        if mask.len() != self.point_count as usize {
            return Err(VisualFieldGpuError::FilterMaskLengthMismatch {
                point_count: self.point_count as usize,
                mask_len: mask.len(),
            });
        }
        Ok(())
    }

    pub fn filter_revision(&self) -> FilterRevision {
        self.filter_revision
    }

    pub(crate) fn count_buffer(&self, index: usize) -> &wgpu::Buffer {
        &self.count_buffers[index]
    }

    pub(crate) fn max_count_buffer(&self) -> &wgpu::Buffer {
        &self.max_count_buffer
    }

    pub fn dataset_revision(&self) -> u64 {
        self.dataset_revision
    }

    pub fn point_capacity(&self) -> usize {
        self.point_capacity
    }

    pub fn active_count_buffer_index(&self) -> usize {
        self.active_count_buffer
    }

    pub(crate) fn grid_generation(&self) -> u64 {
        self.grid_generation
    }

    pub(super) fn resize_grid(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let requested_bins = u64::from(width) * u64::from(height);
        if requested_bins <= self.count_capacity_bins {
            self.grid_width = width;
            self.grid_height = height;
            self.last_max_bin_count = 0;
            return;
        }
        let (counts, max, full, max_readback) = grid_buffers(device, width, height);
        self.count_buffers = counts;
        self.max_count_buffer = max;
        self.full_readback_buffer = full;
        self.max_readback_buffer = max_readback;
        self.compute_bind_groups = bind_groups(
            device,
            &self.compute_bind_group_layout,
            &self.point_buffer,
            &self.filter_mask_buffer,
            &self.params_buffers,
            &self.count_buffers,
            &self.max_count_buffer,
        );
        self.grid_width = width;
        self.grid_height = height;
        self.count_capacity_bins = requested_bins;
        self.active_count_buffer = 0;
        self.last_max_bin_count = 0;
        self.grid_generation += 1;
    }
}
