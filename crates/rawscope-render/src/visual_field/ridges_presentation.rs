//! Static ridge presentation and settled-generation interaction policy.

use std::sync::Arc;

use super::{RidgeFieldGeneration, VisualFieldViewGeneration};

/// A viewport-only reprojection of the last complete ridge generation.
#[derive(Clone)]
pub struct RidgeReprojection {
    pub generation: Arc<RidgeFieldGeneration>,
    pub viewport_revision: u64,
}

/// Interaction state for the static ridge overlay.
///
/// Pointer updates only return a reprojection of `settled`; they cannot call
/// the compute dispatcher. A new ridge generation is submitted separately once
/// the exact source field has settled.
#[derive(Default)]
pub struct RidgeInteraction {
    settled: Option<Arc<RidgeFieldGeneration>>,
    pending_source: Option<VisualFieldViewGeneration>,
    viewport_revision: u64,
}

impl RidgeInteraction {
    pub fn request_settle(&mut self, source_generation: VisualFieldViewGeneration) {
        self.pending_source = Some(source_generation);
    }

    pub fn publish_if_current(&mut self, generation: Arc<RidgeFieldGeneration>) -> bool {
        let source_generation = generation.source.view_generation();
        if self
            .pending_source
            .is_some_and(|pending| pending != source_generation)
        {
            return false;
        }
        self.pending_source = None;
        self.settled = Some(generation);
        true
    }

    pub fn pointer_update(&mut self, viewport_revision: u64) -> Option<RidgeReprojection> {
        self.viewport_revision = viewport_revision;
        self.settled.as_ref().map(|generation| RidgeReprojection {
            generation: Arc::clone(generation),
            viewport_revision,
        })
    }

    pub fn settled(&self) -> Option<Arc<RidgeFieldGeneration>> {
        self.settled.as_ref().map(Arc::clone)
    }

    pub const fn pending_source(&self) -> Option<VisualFieldViewGeneration> {
        self.pending_source
    }

    pub const fn viewport_revision(&self) -> u64 {
        self.viewport_revision
    }
}
