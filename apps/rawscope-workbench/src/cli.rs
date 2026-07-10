//! Minimal command-line parsing for the native workbench.

use std::path::PathBuf;

use rawscope_data::{parse_dataset_profile_id, DatasetProfileId};

use crate::demo::DemoMode;

/// Parsed workbench startup arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkbenchArgs {
    pub(crate) demo_mode: DemoMode,
    pub(crate) input: Option<WorkbenchInput>,
    pub(crate) compare_input: Option<PathBuf>,
}

/// Explicit local dataset binding for a workbench density demo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkbenchInput {
    Scatter {
        path: PathBuf,
        x_column: Option<String>,
        y_column: Option<String>,
        limit: Option<usize>,
        profile: Option<DatasetProfileId>,
    },
    Timeline {
        path: PathBuf,
        time_column: Option<String>,
        lane_column: Option<String>,
        limit: Option<usize>,
        profile: Option<DatasetProfileId>,
    },
}

impl WorkbenchArgs {
    /// Parses CLI args while preserving the existing synthetic scatter default.
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut demo_mode = DemoMode::Scatter;
        let mut input_path = None;
        let mut x_column = None;
        let mut y_column = None;
        let mut time_column = None;
        let mut lane_column = None;
        let mut limit = None;
        let mut compare_input = None;
        let mut profile = None;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            if let Some(value) = arg.strip_prefix("--demo=") {
                demo_mode = DemoMode::from_name(value)?;
                continue;
            }
            if let Some(value) = arg.strip_prefix("--input=") {
                input_path = Some(PathBuf::from(value));
                continue;
            }
            if let Some(value) = arg.strip_prefix("--x=") {
                x_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--y=") {
                y_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--time=") {
                time_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--lane=") {
                lane_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--limit=") {
                limit = Some(parse_limit(value)?);
                continue;
            }
            if let Some(value) = arg.strip_prefix("--compare-input=") {
                compare_input = Some(PathBuf::from(value));
                continue;
            }
            if let Some(value) = arg.strip_prefix("--profile=") {
                profile = Some(parse_dataset_profile_id(value).map_err(|err| err.to_string())?);
                continue;
            }

            match arg.as_str() {
                "--demo" => {
                    let value = next_value(&mut args, "--demo")?;
                    demo_mode = DemoMode::from_name(&value)?;
                }
                "--input" => {
                    input_path = Some(PathBuf::from(next_value(&mut args, "--input")?));
                }
                "--x" => {
                    x_column = Some(next_value(&mut args, "--x")?);
                }
                "--y" => {
                    y_column = Some(next_value(&mut args, "--y")?);
                }
                "--time" => {
                    time_column = Some(next_value(&mut args, "--time")?);
                }
                "--lane" => {
                    lane_column = Some(next_value(&mut args, "--lane")?);
                }
                "--limit" => {
                    let value = next_value(&mut args, "--limit")?;
                    limit = Some(parse_limit(&value)?);
                }
                "--compare-input" => {
                    compare_input = Some(PathBuf::from(next_value(&mut args, "--compare-input")?));
                }
                "--profile" => {
                    let value = next_value(&mut args, "--profile")?;
                    profile =
                        Some(parse_dataset_profile_id(&value).map_err(|err| err.to_string())?);
                }
                _ => {
                    return Err(format!(
                        "unsupported argument '{arg}'; use --demo scatter|timeline, --input, --compare-input, --profile, and explicit column bindings"
                    ));
                }
            }
        }

        let input = match input_path {
            Some(path) if demo_mode.is_scatter() => Some(WorkbenchInput::Scatter {
                path,
                x_column: scatter_x_column(&x_column, &y_column, profile)?,
                y_column: scatter_y_column(&x_column, &y_column, profile)?,
                limit,
                profile,
            }),
            Some(path) if demo_mode.is_timeline() => Some(WorkbenchInput::Timeline {
                path,
                time_column: timeline_time_column(&time_column, &lane_column, profile)?,
                lane_column: timeline_lane_column(&time_column, &lane_column, profile)?,
                limit,
                profile,
            }),
            Some(_) => None,
            None => {
                if compare_input.is_some() {
                    return Err("--compare-input requires --input".to_string());
                }
                if profile.is_some() {
                    return Err("--profile requires --input".to_string());
                }
                reject_column_args_without_input(
                    x_column,
                    y_column,
                    time_column,
                    lane_column,
                    limit,
                    profile,
                )?;
                None
            }
        };

        if input.is_none() && compare_input.is_some() {
            return Err("--compare-input requires --input".to_string());
        }

        Ok(Self {
            demo_mode,
            input,
            compare_input,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value after {flag}"))
}

fn parse_limit(value: &str) -> Result<usize, String> {
    let limit = value
        .parse::<usize>()
        .map_err(|_| format!("--limit must be a positive integer, got '{value}'"))?;
    if limit == 0 {
        return Err("--limit must be greater than zero".to_string());
    }

    Ok(limit)
}

fn reject_column_args_without_input(
    x_column: Option<String>,
    y_column: Option<String>,
    time_column: Option<String>,
    lane_column: Option<String>,
    limit: Option<usize>,
    profile: Option<DatasetProfileId>,
) -> Result<(), String> {
    let has_input_only_args = x_column.is_some()
        || y_column.is_some()
        || time_column.is_some()
        || lane_column.is_some()
        || limit.is_some()
        || profile.is_some();
    if has_input_only_args {
        return Err("--x, --y, --time, --lane, --limit, and --profile require --input".to_string());
    }

    Ok(())
}

fn scatter_x_column(
    x_column: &Option<String>,
    y_column: &Option<String>,
    profile: Option<DatasetProfileId>,
) -> Result<Option<String>, String> {
    require_complete_scatter_override(x_column, y_column, profile)?;
    if profile.is_none() && x_column.is_none() {
        return Err("--input with --demo scatter requires --x <column>".to_string());
    }
    Ok(x_column.clone())
}

fn scatter_y_column(
    x_column: &Option<String>,
    y_column: &Option<String>,
    profile: Option<DatasetProfileId>,
) -> Result<Option<String>, String> {
    require_complete_scatter_override(x_column, y_column, profile)?;
    if profile.is_none() && y_column.is_none() {
        return Err("--input with --demo scatter requires --y <column>".to_string());
    }
    Ok(y_column.clone())
}

fn require_complete_scatter_override(
    x_column: &Option<String>,
    y_column: &Option<String>,
    profile: Option<DatasetProfileId>,
) -> Result<(), String> {
    if profile.is_none() && (x_column.is_none() || y_column.is_none()) {
        return Ok(());
    }
    if x_column.is_some() == y_column.is_some() {
        return Ok(());
    }

    Err(
        "--profile with --demo scatter requires both --x and --y when overriding default bindings"
            .to_string(),
    )
}

fn timeline_time_column(
    time_column: &Option<String>,
    lane_column: &Option<String>,
    profile: Option<DatasetProfileId>,
) -> Result<Option<String>, String> {
    require_complete_timeline_override(time_column, lane_column, profile)?;
    if profile.is_none() && time_column.is_none() {
        return Err("--input with --demo timeline requires --time <column>".to_string());
    }
    Ok(time_column.clone())
}

fn timeline_lane_column(
    time_column: &Option<String>,
    lane_column: &Option<String>,
    profile: Option<DatasetProfileId>,
) -> Result<Option<String>, String> {
    require_complete_timeline_override(time_column, lane_column, profile)?;
    if profile.is_none() && lane_column.is_none() {
        return Err("--input with --demo timeline requires --lane <column>".to_string());
    }
    Ok(lane_column.clone())
}

fn require_complete_timeline_override(
    time_column: &Option<String>,
    lane_column: &Option<String>,
    profile: Option<DatasetProfileId>,
) -> Result<(), String> {
    if profile.is_none() && (time_column.is_none() || lane_column.is_none()) {
        return Ok(());
    }
    if time_column.is_some() == lane_column.is_some() {
        return Ok(());
    }

    Err(
        "--profile with --demo timeline requires both --time and --lane when overriding default bindings"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_synthetic_scatter() {
        let args = WorkbenchArgs::parse(Vec::new()).unwrap();

        assert_eq!(args.demo_mode, DemoMode::Scatter);
        assert_eq!(args.input, None);
        assert_eq!(args.compare_input, None);
    }

    #[test]
    fn parses_scatter_input_binding() {
        let args = WorkbenchArgs::parse([
            "--demo".to_string(),
            "scatter".to_string(),
            "--input".to_string(),
            "data.csv".to_string(),
            "--x".to_string(),
            "latency_ms".to_string(),
            "--y".to_string(),
            "payload_size".to_string(),
            "--limit".to_string(),
            "100".to_string(),
        ])
        .unwrap();

        assert_eq!(
            args.input,
            Some(WorkbenchInput::Scatter {
                path: PathBuf::from("data.csv"),
                x_column: Some("latency_ms".to_string()),
                y_column: Some("payload_size".to_string()),
                limit: Some(100),
                profile: None,
            })
        );
        assert_eq!(args.compare_input, None);
    }

    #[test]
    fn parses_timeline_input_binding() {
        let args = WorkbenchArgs::parse([
            "--demo=timeline".to_string(),
            "--input=events.csv".to_string(),
            "--time=timestamp".to_string(),
            "--lane=provider".to_string(),
        ])
        .unwrap();

        assert_eq!(args.demo_mode, DemoMode::Timeline);
        assert_eq!(
            args.input,
            Some(WorkbenchInput::Timeline {
                path: PathBuf::from("events.csv"),
                time_column: Some("timestamp".to_string()),
                lane_column: Some("provider".to_string()),
                limit: None,
                profile: None,
            })
        );
        assert_eq!(args.compare_input, None);
    }

    #[test]
    fn input_requires_demo_specific_columns() {
        let err = WorkbenchArgs::parse([
            "--demo".to_string(),
            "scatter".to_string(),
            "--input".to_string(),
            "data.csv".to_string(),
            "--x".to_string(),
            "latency_ms".to_string(),
        ])
        .expect_err("scatter input should require y column");

        assert!(err.contains("--y"));
    }

    #[test]
    fn parses_compare_input_in_split_and_equals_forms() {
        let split_args = WorkbenchArgs::parse([
            "--demo".to_string(),
            "scatter".to_string(),
            "--input".to_string(),
            "baseline.csv".to_string(),
            "--compare-input".to_string(),
            "candidate.csv".to_string(),
            "--x".to_string(),
            "latency_ms".to_string(),
            "--y".to_string(),
            "payload_size".to_string(),
        ])
        .unwrap();
        let equals_args = WorkbenchArgs::parse([
            "--demo=timeline".to_string(),
            "--input=baseline.csv".to_string(),
            "--compare-input=candidate.csv".to_string(),
            "--time=timestamp".to_string(),
            "--lane=provider".to_string(),
        ])
        .unwrap();

        assert_eq!(
            split_args.compare_input,
            Some(PathBuf::from("candidate.csv"))
        );
        assert_eq!(
            equals_args.compare_input,
            Some(PathBuf::from("candidate.csv"))
        );
    }

    #[test]
    fn compare_input_requires_primary_input() {
        let err = WorkbenchArgs::parse(["--compare-input=data.csv".to_string()])
            .expect_err("comparison input should require primary input");

        assert!(err.contains("--compare-input requires --input"));
    }

    #[test]
    fn parses_profile_without_explicit_scatter_columns() {
        let args = WorkbenchArgs::parse([
            "--demo=scatter".to_string(),
            "--input=games.csv".to_string(),
            "--profile=lichess-games".to_string(),
        ])
        .unwrap();

        assert_eq!(
            args.input,
            Some(WorkbenchInput::Scatter {
                path: PathBuf::from("games.csv"),
                x_column: None,
                y_column: None,
                limit: None,
                profile: Some(DatasetProfileId::LichessGames),
            })
        );
    }

    #[test]
    fn rejects_unknown_profile_id() {
        let err = WorkbenchArgs::parse([
            "--input=games.csv".to_string(),
            "--profile=unknown-profile".to_string(),
        ])
        .expect_err("unknown profile should fail");

        assert!(err.contains("unknown dataset profile 'unknown-profile'"));
        assert!(err.contains("lichess-games"));
    }
}
