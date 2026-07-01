//! Minimal command-line parsing for the native workbench.

use std::path::PathBuf;

use crate::demo::DemoMode;

/// Parsed workbench startup arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkbenchArgs {
    pub(crate) demo_mode: DemoMode,
    pub(crate) input: Option<WorkbenchInput>,
}

/// Explicit local dataset binding for a workbench density demo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkbenchInput {
    Scatter {
        path: PathBuf,
        x_column: String,
        y_column: String,
        limit: Option<usize>,
    },
    Timeline {
        path: PathBuf,
        time_column: String,
        lane_column: String,
        limit: Option<usize>,
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
                _ => {
                    return Err(format!(
                        "unsupported argument '{arg}'; use --demo scatter|timeline, --input, and explicit column bindings"
                    ));
                }
            }
        }

        let input = match input_path {
            Some(path) if demo_mode.is_scatter() => Some(WorkbenchInput::Scatter {
                path,
                x_column: x_column.ok_or_else(|| {
                    "--input with --demo scatter requires --x <column>".to_string()
                })?,
                y_column: y_column.ok_or_else(|| {
                    "--input with --demo scatter requires --y <column>".to_string()
                })?,
                limit,
            }),
            Some(path) if demo_mode.is_timeline() => Some(WorkbenchInput::Timeline {
                path,
                time_column: time_column.ok_or_else(|| {
                    "--input with --demo timeline requires --time <column>".to_string()
                })?,
                lane_column: lane_column.ok_or_else(|| {
                    "--input with --demo timeline requires --lane <column>".to_string()
                })?,
                limit,
            }),
            Some(_) => None,
            None => {
                reject_column_args_without_input(
                    x_column,
                    y_column,
                    time_column,
                    lane_column,
                    limit,
                )?;
                None
            }
        };

        Ok(Self { demo_mode, input })
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
) -> Result<(), String> {
    let has_input_only_args = x_column.is_some()
        || y_column.is_some()
        || time_column.is_some()
        || lane_column.is_some()
        || limit.is_some();
    if has_input_only_args {
        return Err("--x, --y, --time, --lane, and --limit require --input".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_synthetic_scatter() {
        let args = WorkbenchArgs::parse(Vec::new()).unwrap();

        assert_eq!(args.demo_mode, DemoMode::Scatter);
        assert_eq!(args.input, None);
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
                x_column: "latency_ms".to_string(),
                y_column: "payload_size".to_string(),
                limit: Some(100),
            })
        );
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
                time_column: "timestamp".to_string(),
                lane_column: "provider".to_string(),
                limit: None,
            })
        );
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
}
