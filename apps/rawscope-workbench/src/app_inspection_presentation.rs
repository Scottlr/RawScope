//! Workbench lifecycle for settled inspection tooltip/focus presentation.

use crate::{
    app::WorkbenchApp,
    ui_inspection_tooltip::{
        InspectionPresentationFrame, InspectionPresentationKey, InspectionPresentationMotion,
    },
    ui_scatter_inspection::{scatter_inspection_tooltip_state, ScatterInspectionUiState},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct InspectionPresentationState {
    motion: InspectionPresentationMotion,
    retained_content: Option<ScatterInspectionUiState>,
    frame: InspectionPresentationFrame,
}

impl InspectionPresentationState {
    pub(crate) fn sync(
        &mut self,
        content: Option<ScatterInspectionUiState>,
        key: Option<InspectionPresentationKey>,
        now_ms: u64,
        reduced_motion: bool,
    ) {
        if content
            .as_ref()
            .is_some_and(|content| content.hovered.is_some())
        {
            self.retained_content = content;
        }
        self.motion.sync(key, now_ms, reduced_motion);
        self.frame = self.motion.frame(now_ms, reduced_motion);
        if key.is_none() && !self.frame.running && self.motion.is_hidden() {
            self.retained_content = None;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.motion = InspectionPresentationMotion::default();
        self.retained_content = None;
        self.frame = InspectionPresentationFrame {
            opacity: 0.0,
            translate_y_px: 0.0,
            running: false,
        };
    }

    pub(crate) fn frame(&self) -> InspectionPresentationFrame {
        self.frame
    }

    pub(crate) fn retained_content(&self) -> Option<&ScatterInspectionUiState> {
        self.retained_content.as_ref()
    }
}

impl WorkbenchApp {
    pub(crate) fn update_inspection_presentation(&mut self) {
        let content = scatter_inspection_tooltip_state(self);
        let key = content.as_ref().and_then(|state| {
            let hit = state.hovered.as_ref()?;
            Some(InspectionPresentationKey {
                viewport_revision: self.scatter_inspection.cache_viewport_revision,
                filter_revision: self.scatter_inspection.cache_filter_revision,
                bin_x: hit.bin_x,
                bin_y: hit.bin_y,
                density_mode: self.scatter.density_mode,
            })
        });
        self.inspection_presentation.sync(
            content,
            key,
            self.render_schedule.elapsed_ms(),
            self.visual_transition.config.reduced_motion,
        );
    }

    pub(crate) fn clear_inspection_presentation(&mut self) {
        self.inspection_presentation.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_bin_pointer_motion_keeps_presentation_key_stable() {
        let key = InspectionPresentationKey {
            viewport_revision: 2,
            filter_revision: Default::default(),
            bin_x: 4,
            bin_y: 6,
            density_mode: rawscope_render::ScatterDensityMode::AbsoluteDensity,
        };
        let mut motion = InspectionPresentationMotion::default();
        motion.sync(Some(key), 0, false);
        let first = motion.frame(40, false);
        motion.sync(Some(key), 70, false);
        let second = motion.frame(70, false);

        assert!(first.running);
        assert!(second.running);
        assert!(second.opacity >= first.opacity);
    }
}
