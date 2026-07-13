use std::path::PathBuf;

use rawscope_data::DatasetProfileId;

use super::{VisualFieldProjectionKind, WorkbenchArgs, WorkbenchInput};
use crate::demo::DemoMode;

#[test]
fn defaults_to_synthetic_scatter() {
    let args = WorkbenchArgs::parse(Vec::new()).unwrap();

    assert_eq!(args.demo_mode, DemoMode::Scatter);
    assert_eq!(args.input, None);
    assert_eq!(args.compare_input, None);
    assert_eq!(args.session_path, None);
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
            category_column: None,
            projection: VisualFieldProjectionKind::NumericPair,
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
            category_column: None,
            projection: VisualFieldProjectionKind::NumericPair,
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

#[test]
fn parses_session_in_split_and_equals_forms() {
    let split_args = WorkbenchArgs::parse([
        "--session".to_string(),
        "analysis.rawscope.json".to_string(),
    ])
    .unwrap();
    let equals_args =
        WorkbenchArgs::parse(["--session=timeline.rawscope.json".to_string()]).unwrap();

    assert_eq!(
        split_args.session_path,
        Some(PathBuf::from("analysis.rawscope.json"))
    );
    assert_eq!(
        equals_args.session_path,
        Some(PathBuf::from("timeline.rawscope.json"))
    );
    assert_eq!(split_args.input, None);
    assert_eq!(equals_args.compare_input, None);
}

#[test]
fn session_rejects_direct_startup_overrides() {
    let err = WorkbenchArgs::parse([
        "--session=analysis.rawscope.json".to_string(),
        "--demo".to_string(),
        "scatter".to_string(),
    ])
    .expect_err("session should be authoritative");

    assert!(err.contains("--session cannot be combined"));
}
