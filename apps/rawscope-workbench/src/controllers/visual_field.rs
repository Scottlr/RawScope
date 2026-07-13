//! Generic visual-field mode availability and command ownership.
//!
//! This module deliberately owns only workbench state.  Mapping validity and
//! mode compatibility remain analysis contracts, while render resources stay
//! with `rawscope-render`.  A command either changes the resource-producing
//! intent, changes presentation uniforms, or is an idempotent no-op.

use rawscope_analysis::visual_field::{
    RidgeScale, VisualFieldMapping, VisualFieldMode, VisualFieldModeSupport,
};
use rawscope_render::{ComparisonPresentation, ComparisonSplit, ScatterDensityPresentation};

/// Availability of the comparison cohort required by the comparison mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComparisonCohortState {
    Missing,
    Empty,
    Ready,
}

/// Device and reservation capability for optional derived visual fields.
///
/// Exact density retains its documented fallback when either capability is
/// unavailable.  The flags therefore apply to composition, comparison, and
/// ridge modes rather than making the base density view disappear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VisualFieldCapabilities {
    pub(crate) device_limit_ok: bool,
    pub(crate) resource_budget_ok: bool,
}

impl VisualFieldCapabilities {
    pub(crate) const fn new(device_limit_ok: bool, resource_budget_ok: bool) -> Self {
        Self {
            device_limit_ok,
            resource_budget_ok,
        }
    }

    pub(crate) const fn available() -> Self {
        Self::new(true, true)
    }

    pub(crate) const fn device_limited() -> Self {
        Self::new(false, true)
    }

    pub(crate) const fn resource_limited() -> Self {
        Self::new(true, false)
    }
}

impl Default for VisualFieldCapabilities {
    fn default() -> Self {
        Self::available()
    }
}

/// Readiness of the settled field used by the current workbench view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualFieldReadiness {
    Ready,
    Preparing,
}

/// Typed reason a compatible mode cannot currently be enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualModeUnavailableReason {
    ProjectionIncompatible,
    CategoryNotMapped,
    ComparisonCohortMissing,
    ComparisonCohortEmpty,
    DeviceLimit,
    ResourceBudget,
}

/// Availability projected into controls and the plot freshness indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualModeAvailability {
    Available,
    Preparing,
    Unavailable(VisualModeUnavailableReason),
}

/// Applies the closed compatibility table and typed runtime prerequisites.
///
/// The analysis mapping's `supports` result is the only projection/mode rule.
/// Device and budget failures disable only derived modes; exact density keeps
/// its fallback path.  Readiness is evaluated last so an unavailable reason
/// is never hidden behind a generic preparing state.
pub(crate) fn visual_mode_availability(
    mode: VisualFieldMode,
    mapping: &VisualFieldMapping,
    comparison: ComparisonCohortState,
    capabilities: VisualFieldCapabilities,
    readiness: VisualFieldReadiness,
) -> VisualModeAvailability {
    if matches!(
        mapping.supports(mode),
        VisualFieldModeSupport::CategoryRequired
    ) {
        return VisualModeAvailability::Unavailable(VisualModeUnavailableReason::CategoryNotMapped);
    }

    if mode == VisualFieldMode::CohortComparison {
        match comparison {
            ComparisonCohortState::Missing => {
                return VisualModeAvailability::Unavailable(
                    VisualModeUnavailableReason::ComparisonCohortMissing,
                );
            }
            ComparisonCohortState::Empty => {
                return VisualModeAvailability::Unavailable(
                    VisualModeUnavailableReason::ComparisonCohortEmpty,
                );
            }
            ComparisonCohortState::Ready => {}
        }
    }

    let derived_mode = !matches!(mode, VisualFieldMode::Density);
    if derived_mode && !capabilities.device_limit_ok {
        return VisualModeAvailability::Unavailable(VisualModeUnavailableReason::DeviceLimit);
    }
    if derived_mode && !capabilities.resource_budget_ok {
        return VisualModeAvailability::Unavailable(VisualModeUnavailableReason::ResourceBudget);
    }

    match readiness {
        VisualFieldReadiness::Ready => VisualModeAvailability::Available,
        VisualFieldReadiness::Preparing => VisualModeAvailability::Preparing,
    }
}

/// Typed workbench actions for the canonical visual-field controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VisualFieldCommand {
    SetMapping(VisualFieldMapping),
    SetMode(VisualFieldMode),
    SetDensityPresentation(ScatterDensityPresentation),
    SetComparisonPresentation(ComparisonPresentation),
    SetComparisonSplit(ComparisonSplit),
    SetRidgeScale(RidgeScale),
}

/// The canonical visual-field state projected to app/UI owners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFieldControllerState {
    mapping: VisualFieldMapping,
    mode: VisualFieldMode,
    density_presentation: ScatterDensityPresentation,
    comparison_presentation: ComparisonPresentation,
    comparison_split: ComparisonSplit,
    ridge_scale: RidgeScale,
}

impl VisualFieldControllerState {
    fn new(mapping: VisualFieldMapping) -> Self {
        Self {
            mapping,
            mode: VisualFieldMode::Density,
            density_presentation: ScatterDensityPresentation::ExactCells,
            comparison_presentation: ComparisonPresentation::SignedDifference,
            comparison_split: ComparisonSplit::default(),
            ridge_scale: RidgeScale::Medium,
        }
    }

    pub(crate) const fn mapping(&self) -> VisualFieldMapping {
        self.mapping
    }

    pub(crate) const fn mode(&self) -> VisualFieldMode {
        self.mode
    }

    pub(crate) const fn density_presentation(&self) -> ScatterDensityPresentation {
        self.density_presentation
    }

    pub(crate) const fn comparison_presentation(&self) -> ComparisonPresentation {
        self.comparison_presentation
    }

    pub(crate) const fn comparison_split(&self) -> ComparisonSplit {
        self.comparison_split
    }

    pub(crate) const fn ridge_scale(&self) -> RidgeScale {
        self.ridge_scale
    }
}

/// Classification returned after applying a typed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualFieldCommandResult {
    Noop,
    PresentationOnly,
    ResourceIntentChanged { generation: u64 },
    Rejected(VisualModeUnavailableReason),
}

/// Owns one source of truth for mapping, mode, presentation, and generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFieldController {
    state: VisualFieldControllerState,
    resource_generation: u64,
}

impl VisualFieldController {
    pub(crate) fn new(mapping: VisualFieldMapping) -> Self {
        Self {
            state: VisualFieldControllerState::new(mapping),
            resource_generation: 0,
        }
    }

    pub(crate) const fn state(&self) -> VisualFieldControllerState {
        self.state
    }

    pub(crate) const fn resource_generation(&self) -> u64 {
        self.resource_generation
    }

    pub(crate) fn handle(&mut self, command: VisualFieldCommand) -> VisualFieldCommandResult {
        match command {
            VisualFieldCommand::SetMapping(mapping) => {
                if matches!(
                    mapping.supports(self.state.mode),
                    VisualFieldModeSupport::CategoryRequired
                ) {
                    return VisualFieldCommandResult::Rejected(
                        VisualModeUnavailableReason::CategoryNotMapped,
                    );
                }
                if mapping == self.state.mapping {
                    return VisualFieldCommandResult::Noop;
                }
                self.state.mapping = mapping;
                self.bump_resource_generation()
            }
            VisualFieldCommand::SetMode(mode) => {
                if matches!(
                    self.state.mapping.supports(mode),
                    VisualFieldModeSupport::CategoryRequired
                ) {
                    return VisualFieldCommandResult::Rejected(
                        VisualModeUnavailableReason::CategoryNotMapped,
                    );
                }
                if mode == self.state.mode {
                    return VisualFieldCommandResult::Noop;
                }
                self.state.mode = mode;
                self.bump_resource_generation()
            }
            VisualFieldCommand::SetDensityPresentation(presentation) => {
                if presentation == self.state.density_presentation {
                    VisualFieldCommandResult::Noop
                } else {
                    self.state.density_presentation = presentation;
                    VisualFieldCommandResult::PresentationOnly
                }
            }
            VisualFieldCommand::SetComparisonPresentation(presentation) => {
                if presentation == self.state.comparison_presentation {
                    VisualFieldCommandResult::Noop
                } else {
                    self.state.comparison_presentation = presentation;
                    VisualFieldCommandResult::PresentationOnly
                }
            }
            VisualFieldCommand::SetComparisonSplit(split) => {
                if split == self.state.comparison_split {
                    VisualFieldCommandResult::Noop
                } else {
                    self.state.comparison_split = split;
                    VisualFieldCommandResult::PresentationOnly
                }
            }
            VisualFieldCommand::SetRidgeScale(scale) => {
                if scale == self.state.ridge_scale {
                    VisualFieldCommandResult::Noop
                } else {
                    self.state.ridge_scale = scale;
                    self.bump_resource_generation()
                }
            }
        }
    }

    fn bump_resource_generation(&mut self) -> VisualFieldCommandResult {
        self.resource_generation = self.resource_generation.saturating_add(1);
        VisualFieldCommandResult::ResourceIntentChanged {
            generation: self.resource_generation,
        }
    }
}

#[cfg(test)]
#[path = "visual_field_tests.rs"]
mod tests;
