//! Workbench coordination for density encoding and scatter presentation controls.

use rawscope_render::{DensityTransform, ScatterDensityPresentation};
use tracing::error;

use crate::{app::WorkbenchApp, demo::DemoMode, ui::WorkbenchSurface};

impl WorkbenchApp {
    pub(crate) fn set_density_transform(&mut self, transform: DensityTransform) {
        if self.visible_surface != WorkbenchSurface::Primary {
            return;
        }

        let recompute_result = match self.demo_mode {
            DemoMode::Scatter => {
                let next_encoding = self.scatter.density_encoding.with_transform(transform);
                if self.scatter.density_encoding == next_encoding {
                    return;
                }
                self.scatter.density_encoding = next_encoding;
                self.recompute_density()
            }
            DemoMode::Timeline => {
                let next_encoding = self.timeline.density_encoding.with_transform(transform);
                if self.timeline.density_encoding == next_encoding {
                    return;
                }
                self.timeline.density_encoding = next_encoding;
                self.recompute_timeline_density()
            }
        };

        if let Err(err) = recompute_result {
            error!(
                error = %err,
                transform = transform.label(),
                "failed to recompute density after transform change"
            );
        }
    }

    pub(crate) fn set_scatter_density_presentation(
        &mut self,
        presentation: ScatterDensityPresentation,
    ) {
        if self.visible_surface != WorkbenchSurface::Primary || !self.demo_mode.is_scatter() {
            return;
        }
        if self.scatter.density_presentation == presentation {
            return;
        }

        self.scatter.density_presentation = presentation;
        if let Err(err) = self.recompute_density() {
            error!(
                error = %err,
                presentation = presentation.evidence_label(),
                "failed to recompute scatter density after presentation change"
            );
        }
    }
}
