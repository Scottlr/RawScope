//! Pure responsive projection for the fixed RawScope analytical shell.

use crate::ui_theme::{WorkbenchChromeMetrics, CHROME_METRICS};

/// User preference for one closed shell region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellRegionState {
    Collapsed,
    Expanded,
}

impl Default for ShellRegionState {
    fn default() -> Self {
        Self::Expanded
    }
}

/// Presentation state derived from available logical space and user preference.
///
/// The projection never mutates the preference input.  A later frame with
/// sufficient space therefore restores the user's chosen region states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResponsiveShellProjection {
    pub(crate) activity_rail: ShellRegionState,
    pub(crate) inspector: ShellRegionState,
    pub(crate) show_secondary_text: bool,
    pub(crate) plot_width_points: f32,
    pub(crate) plot_height_points: f32,
    pub(crate) plot_is_usable: bool,
}

/// Projects the fixed shell without introducing a docking tree or a second
/// plot-geometry owner.  Secondary text yields first, then the inspector, then
/// the activity rail; the central plot is never allowed below named minima
/// when a valid collapsed presentation can avoid it.
pub(crate) fn project_responsive_shell(
    available_width_points: f32,
    available_height_points: f32,
    preferred_activity_rail: ShellRegionState,
    preferred_inspector: ShellRegionState,
) -> ResponsiveShellProjection {
    project_responsive_shell_with_metrics(
        available_width_points,
        available_height_points,
        preferred_activity_rail,
        preferred_inspector,
        CHROME_METRICS,
    )
}

fn project_responsive_shell_with_metrics(
    available_width_points: f32,
    available_height_points: f32,
    preferred_activity_rail: ShellRegionState,
    preferred_inspector: ShellRegionState,
    metrics: WorkbenchChromeMetrics,
) -> ResponsiveShellProjection {
    let available_width_points = available_width_points.max(0.0);
    let available_height_points = available_height_points.max(0.0);
    let mut activity_rail = preferred_activity_rail;
    let mut inspector = preferred_inspector;
    let mut show_secondary_text = true;

    let plot_height_points = (available_height_points
        - metrics.outer_margin_points * 2.0
        - metrics.command_bar_height_points
        - metrics.status_bar_height_points
        - metrics.region_gap_points * 2.0)
        .max(0.0);

    let plot_width = |activity_rail: ShellRegionState, inspector: ShellRegionState| {
        let activity_width = match activity_rail {
            ShellRegionState::Collapsed => metrics.activity_rail_collapsed_width_points,
            ShellRegionState::Expanded => metrics.activity_rail_width_points,
        };
        let inspector_width = match inspector {
            ShellRegionState::Collapsed => metrics.activity_rail_collapsed_width_points,
            ShellRegionState::Expanded => metrics.inspector_width_points,
        };
        available_width_points
            - metrics.outer_margin_points * 2.0
            - activity_width
            - inspector_width
            - metrics.region_gap_points * 2.0
    };

    let mut plot_width_points = plot_width(activity_rail, inspector);
    if plot_width_points < metrics.minimum_plot_width_points
        || plot_height_points < metrics.minimum_plot_height_points
    {
        show_secondary_text = false;
    }
    if plot_width_points < metrics.minimum_plot_width_points
        && inspector == ShellRegionState::Expanded
    {
        inspector = ShellRegionState::Collapsed;
        plot_width_points = plot_width(activity_rail, inspector);
    }
    if plot_width_points < metrics.minimum_plot_width_points
        && activity_rail == ShellRegionState::Expanded
    {
        activity_rail = ShellRegionState::Collapsed;
        plot_width_points = plot_width(activity_rail, inspector);
    }

    plot_width_points = plot_width_points.max(0.0);
    let plot_is_usable = plot_width_points >= metrics.minimum_plot_width_points
        && plot_height_points >= metrics.minimum_plot_height_points;
    ResponsiveShellProjection {
        activity_rail,
        inspector,
        show_secondary_text,
        plot_width_points,
        plot_height_points,
        plot_is_usable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_layout_collapses_secondary_regions_before_plot() {
        let projection = project_responsive_shell(
            475.0,
            720.0,
            ShellRegionState::Expanded,
            ShellRegionState::Expanded,
        );

        assert!(!projection.show_secondary_text);
        assert_eq!(projection.inspector, ShellRegionState::Collapsed);
        assert_eq!(projection.activity_rail, ShellRegionState::Collapsed);
        assert!(projection.plot_width_points >= CHROME_METRICS.minimum_plot_width_points);
    }

    #[test]
    fn responsive_collapse_preserves_user_region_preference() {
        let narrow = project_responsive_shell(
            475.0,
            720.0,
            ShellRegionState::Expanded,
            ShellRegionState::Expanded,
        );
        let wide = project_responsive_shell(
            1_600.0,
            900.0,
            ShellRegionState::Expanded,
            ShellRegionState::Expanded,
        );

        assert_eq!(narrow.inspector, ShellRegionState::Collapsed);
        assert_eq!(narrow.activity_rail, ShellRegionState::Collapsed);
        assert_eq!(wide.inspector, ShellRegionState::Expanded);
        assert_eq!(wide.activity_rail, ShellRegionState::Expanded);
    }

    #[test]
    fn unusable_area_is_reported_without_negative_geometry() {
        let projection = project_responsive_shell(
            300.0,
            200.0,
            ShellRegionState::Expanded,
            ShellRegionState::Expanded,
        );

        assert!(!projection.plot_is_usable);
        assert!(projection.plot_width_points >= 0.0);
        assert!(projection.plot_height_points >= 0.0);
    }
}
