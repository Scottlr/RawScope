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
    pub(crate) session_path: Option<PathBuf>,
}

/// Explicit local dataset binding for a workbench density demo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkbenchInput {
    Scatter {
        path: PathBuf,
        x_column: Option<String>,
        y_column: Option<String>,
        category_column: Option<String>,
        projection: VisualFieldProjectionKind,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualFieldProjectionKind {
    NumericPair,
    TimeValue,
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
        let mut session_path = None;
        let mut saw_direct_startup_flag = false;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            if let Some(value) = arg.strip_prefix("--demo=") {
                saw_direct_startup_flag = true;
                demo_mode = DemoMode::from_name(value)?;
                continue;
            }
            if let Some(value) = arg.strip_prefix("--input=") {
                saw_direct_startup_flag = true;
                input_path = Some(PathBuf::from(value));
                continue;
            }
            if let Some(value) = arg.strip_prefix("--x=") {
                saw_direct_startup_flag = true;
                x_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--y=") {
                saw_direct_startup_flag = true;
                y_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--time=") {
                saw_direct_startup_flag = true;
                time_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--lane=") {
                saw_direct_startup_flag = true;
                lane_column = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--limit=") {
                saw_direct_startup_flag = true;
                limit = Some(parse_limit(value)?);
                continue;
            }
            if let Some(value) = arg.strip_prefix("--compare-input=") {
                saw_direct_startup_flag = true;
                compare_input = Some(PathBuf::from(value));
                continue;
            }
            if let Some(value) = arg.strip_prefix("--profile=") {
                saw_direct_startup_flag = true;
                profile = Some(parse_dataset_profile_id(value).map_err(|err| err.to_string())?);
                continue;
            }
            if let Some(value) = arg.strip_prefix("--session=") {
                if session_path.is_some() {
                    return Err("--session may only be supplied once".to_string());
                }
                session_path = Some(PathBuf::from(value));
                continue;
            }

            match arg.as_str() {
                "--demo" => {
                    saw_direct_startup_flag = true;
                    let value = next_value(&mut args, "--demo")?;
                    demo_mode = DemoMode::from_name(&value)?;
                }
                "--input" => {
                    saw_direct_startup_flag = true;
                    input_path = Some(PathBuf::from(next_value(&mut args, "--input")?));
                }
                "--x" => {
                    saw_direct_startup_flag = true;
                    x_column = Some(next_value(&mut args, "--x")?);
                }
                "--y" => {
                    saw_direct_startup_flag = true;
                    y_column = Some(next_value(&mut args, "--y")?);
                }
                "--time" => {
                    saw_direct_startup_flag = true;
                    time_column = Some(next_value(&mut args, "--time")?);
                }
                "--lane" => {
                    saw_direct_startup_flag = true;
                    lane_column = Some(next_value(&mut args, "--lane")?);
                }
                "--limit" => {
                    saw_direct_startup_flag = true;
                    let value = next_value(&mut args, "--limit")?;
                    limit = Some(parse_limit(&value)?);
                }
                "--compare-input" => {
                    saw_direct_startup_flag = true;
                    compare_input = Some(PathBuf::from(next_value(&mut args, "--compare-input")?));
                }
                "--profile" => {
                    saw_direct_startup_flag = true;
                    let value = next_value(&mut args, "--profile")?;
                    profile =
                        Some(parse_dataset_profile_id(&value).map_err(|err| err.to_string())?);
                }
                "--session" => {
                    if session_path.is_some() {
                        return Err("--session may only be supplied once".to_string());
                    }
                    session_path = Some(PathBuf::from(next_value(&mut args, "--session")?));
                }
                _ => {
                    return Err(format!(
                        "unsupported argument '{arg}'; use --demo scatter|timeline, --session, --input, --compare-input, --profile, and explicit column bindings"
                    ));
                }
            }
        }

        if session_path.is_some() {
            if saw_direct_startup_flag {
                return Err(
                    "--session cannot be combined with --demo, --input, --compare-input, --profile, --limit, or explicit column bindings"
                        .to_string(),
                );
            }
            return Ok(Self {
                demo_mode,
                input: None,
                compare_input: None,
                session_path,
            });
        }

        let input = match input_path {
            Some(path) if demo_mode.is_scatter() => Some(WorkbenchInput::Scatter {
                path,
                x_column: scatter_x_column(&x_column, &y_column, profile)?,
                y_column: scatter_y_column(&x_column, &y_column, profile)?,
                category_column: None,
                projection: VisualFieldProjectionKind::NumericPair,
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
            session_path: None,
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
#[path = "cli_tests.rs"]
mod tests;
