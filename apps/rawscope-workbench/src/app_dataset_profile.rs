//! Workbench-owned dataset profile validation and default binding resolution.

use std::{io, path::Path};

use rawscope_data::{
    dataset_profile, load_dataset_schema, validate_dataset_profile, DatasetProfileId,
};
use rawscope_render::{
    PointRevealConfig, ReliefFieldConfig, ScatterDensityMode, ScatterDensityPresentation,
};

use crate::{app::WorkbenchApp, app_interaction_mode::WorkbenchInteractionMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedScatterInputBinding {
    pub(crate) x_column: String,
    pub(crate) y_column: String,
    pub(crate) active_profile: Option<DatasetProfileId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedTimelineInputBinding {
    pub(crate) time_column: String,
    pub(crate) lane_column: String,
    pub(crate) active_profile: Option<DatasetProfileId>,
}

impl WorkbenchApp {
    pub(crate) fn apply_scatter_profile_defaults(&mut self, profile_id: DatasetProfileId) {
        match profile_id {
            DatasetProfileId::LichessGames => {
                self.scatter.density_mode = ScatterDensityMode::AbsoluteDensity;
                self.scatter.density_presentation = ScatterDensityPresentation::TopographicField;
                self.scatter.relief_config = ReliefFieldConfig::default();
                self.point_reveal.config = PointRevealConfig::default();
                self.interaction_mode = WorkbenchInteractionMode::Pan;
                self.scatter_filters.filters.clear();
            }
        }
    }
}

pub(crate) fn resolve_scatter_input_binding(
    path: &Path,
    x_column: Option<&str>,
    y_column: Option<&str>,
    limit: Option<usize>,
    profile_id: Option<DatasetProfileId>,
) -> Result<ResolvedScatterInputBinding, io::Error> {
    if let Some(profile_id) = profile_id {
        validate_profile_schema(path, limit, profile_id)?;
        if let (Some(x_column), Some(y_column)) = (x_column, y_column) {
            return Ok(ResolvedScatterInputBinding {
                x_column: x_column.to_string(),
                y_column: y_column.to_string(),
                active_profile: Some(profile_id),
            });
        }

        let binding = dataset_profile(profile_id)
            .scatter_binding
            .ok_or_else(|| missing_default_binding_error(profile_id, "scatter"))?;
        return Ok(ResolvedScatterInputBinding {
            x_column: binding.x_column.to_string(),
            y_column: binding.y_column.to_string(),
            active_profile: Some(profile_id),
        });
    }

    let x_column = x_column.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            missing_scatter_binding_message(),
        )
    })?;
    let y_column = y_column.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            missing_scatter_binding_message(),
        )
    })?;
    Ok(ResolvedScatterInputBinding {
        x_column: x_column.to_string(),
        y_column: y_column.to_string(),
        active_profile: None,
    })
}

pub(crate) fn resolve_timeline_input_binding(
    path: &Path,
    time_column: Option<&str>,
    lane_column: Option<&str>,
    limit: Option<usize>,
    profile_id: Option<DatasetProfileId>,
) -> Result<ResolvedTimelineInputBinding, io::Error> {
    if let Some(profile_id) = profile_id {
        validate_profile_schema(path, limit, profile_id)?;
        if let (Some(time_column), Some(lane_column)) = (time_column, lane_column) {
            return Ok(ResolvedTimelineInputBinding {
                time_column: time_column.to_string(),
                lane_column: lane_column.to_string(),
                active_profile: Some(profile_id),
            });
        }

        let binding = dataset_profile(profile_id)
            .timeline_binding
            .ok_or_else(|| missing_default_binding_error(profile_id, "timeline"))?;
        return Ok(ResolvedTimelineInputBinding {
            time_column: binding.time_column.to_string(),
            lane_column: binding.lane_column.to_string(),
            active_profile: Some(profile_id),
        });
    }

    let time_column = time_column.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            missing_timeline_binding_message(),
        )
    })?;
    let lane_column = lane_column.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            missing_timeline_binding_message(),
        )
    })?;
    Ok(ResolvedTimelineInputBinding {
        time_column: time_column.to_string(),
        lane_column: lane_column.to_string(),
        active_profile: None,
    })
}

fn validate_profile_schema(
    path: &Path,
    limit: Option<usize>,
    profile_id: DatasetProfileId,
) -> Result<(), io::Error> {
    let profile = dataset_profile(profile_id);
    let schema = load_dataset_schema(path, limit).map_err(|source| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to load schema for dataset profile '{}' from '{}': {source}",
                profile_id,
                path.display()
            ),
        )
    })?;
    validate_dataset_profile(profile, &schema).map_err(|source| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to validate dataset profile '{}' for '{}': {source}",
                profile_id,
                path.display()
            ),
        )
    })
}

fn missing_default_binding_error(profile_id: DatasetProfileId, demo_name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "dataset profile '{}' does not define a default {demo_name} binding",
            profile_id
        ),
    )
}

fn missing_scatter_binding_message() -> &'static str {
    "scatter input requires either --x/--y or a profile with default scatter bindings"
}

fn missing_timeline_binding_message() -> &'static str {
    "timeline input requires either --time/--lane or a profile with default timeline bindings"
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rawscope_data::{load_scatter_dataset, DatasetProfileId};

    use super::*;
    use crate::{
        app::WorkbenchApp,
        app_session::resolve_workbench_startup,
        cli::{WorkbenchArgs, WorkbenchInput},
        demo::DemoMode,
    };

    #[test]
    fn profile_applies_default_bindings_after_local_load() {
        let path = write_csv_fixture(
            "profile-default-bindings",
            "created_at,white_rating,black_rating,winner\n1,1500,1600,white\n2,1700,1680,black\n",
        );

        let resolved = resolve_scatter_input_binding(
            &path,
            None,
            None,
            None,
            Some(DatasetProfileId::LichessGames),
        )
        .unwrap();
        let dataset =
            load_scatter_dataset(&path, &resolved.x_column, &resolved.y_column, None).unwrap();

        assert_eq!(
            resolved.active_profile,
            Some(DatasetProfileId::LichessGames)
        );
        assert_eq!(dataset.x_column, "white_rating");
        assert_eq!(dataset.y_column, "black_rating");
        remove_fixture(&path);
    }

    #[test]
    fn profile_error_lists_missing_columns() {
        let path = write_csv_fixture(
            "profile-missing-columns",
            "created_at,white_rating\n1,1500\n2,1700\n",
        );

        let err = resolve_scatter_input_binding(
            &path,
            None,
            None,
            None,
            Some(DatasetProfileId::LichessGames),
        )
        .expect_err("profile validation should fail");

        assert!(err.to_string().contains("black_rating"));
        assert!(err.to_string().contains("winner"));
        remove_fixture(&path);
    }

    #[test]
    fn comparison_input_does_not_override_primary_profile() {
        let primary_path = write_csv_fixture(
            "profile-primary",
            "created_at,white_rating,black_rating,winner\n1,1500,1600,white\n2,1700,1680,black\n",
        );
        let compare_path = write_csv_fixture(
            "profile-compare",
            "created_at,white_rating,black_rating,winner\n3,1800,1750,draw\n",
        );
        let args = WorkbenchArgs {
            demo_mode: DemoMode::Scatter,
            input: Some(WorkbenchInput::Scatter {
                path: primary_path.clone(),
                x_column: None,
                y_column: None,
                limit: None,
                profile: Some(DatasetProfileId::LichessGames),
            }),
            compare_input: Some(compare_path.clone()),
            session_path: None,
        };
        let mut app = WorkbenchApp::new(resolve_workbench_startup(args).unwrap());

        let WorkbenchInput::Scatter {
            path,
            x_column,
            y_column,
            limit,
            profile,
        } = app.input.clone().expect("scatter input should exist")
        else {
            panic!("expected scatter input");
        };
        let resolved = resolve_scatter_input_binding(
            &path,
            x_column.as_deref(),
            y_column.as_deref(),
            limit,
            profile,
        )
        .unwrap();
        app.workbench_state.active_dataset_profile = resolved.active_profile;
        let comparison_rows = app
            .load_scatter_comparison_source_rows(&resolved.x_column, &resolved.y_column, limit)
            .unwrap();
        app.set_comparison_source_rows(comparison_rows);

        assert_eq!(
            app.workbench_state.active_dataset_profile,
            Some(DatasetProfileId::LichessGames)
        );
        assert!(app.comparison_source_rows.is_some());
        remove_fixture(&primary_path);
        remove_fixture(&compare_path);
    }

    fn write_csv_fixture(name: &str, contents: &str) -> PathBuf {
        let path = fixture_path(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn remove_fixture(path: &PathBuf) {
        fs::remove_file(path).unwrap();
    }

    fn fixture_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("rawscope-{name}-{}.csv", std::process::id()));
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        path
    }

    #[test]
    fn lichess_defaults_start_raw_unfiltered_and_topographic() {
        let mut app = WorkbenchApp::default();
        app.apply_scatter_profile_defaults(DatasetProfileId::LichessGames);

        assert_eq!(
            app.scatter_projection.active,
            rawscope_data::ScatterProjection::RawXY
        );
        assert!(!app.scatter_filters.filters.is_active());
        assert_eq!(
            app.scatter.density_presentation,
            ScatterDensityPresentation::TopographicField
        );
        assert_eq!(
            app.scatter.density_mode,
            ScatterDensityMode::AbsoluteDensity
        );
        assert_eq!(
            app.point_reveal.config.mode,
            rawscope_render::PointRevealMode::Auto
        );
        assert_eq!(app.interaction_mode, WorkbenchInteractionMode::Pan);
    }
}
