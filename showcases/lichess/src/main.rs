//! Prepare and launch the flagship Lichess view through RawScope's public API.

use std::{env, error::Error, ffi::OsString, path::PathBuf, process::ExitCode};

use rawscope::LocalCsvSession;

const DEFAULT_SESSION_DIRECTORY: &str = ".showcase-data/lichess/session";
const LICHESS_PROFILE: &str = "lichess-games";
const LICHESS_DISPLAY_NAME: &str = "Lichess games";
const WHITE_RATING_COLUMN: &str = "white_rating";
const BLACK_RATING_COLUMN: &str = "black_rating";
const GAME_ID_COLUMN: &str = "game_id";
const USAGE: &str = "Usage:\n  rawscope-showcase-lichess prepare <games.csv> [session-dir]\n  rawscope-showcase-lichess run <games.csv> [session-dir]";

fn main() -> ExitCode {
    match run(env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(mut arguments: impl Iterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    let command = arguments.next().ok_or(USAGE)?;
    let launch = if command == "prepare" {
        false
    } else if command == "run" {
        true
    } else {
        return Err(USAGE.into());
    };
    let dataset_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let destination = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SESSION_DIRECTORY));
    if arguments.next().is_some() {
        return Err(USAGE.into());
    }

    let prepared = LocalCsvSession::scatter(dataset_path, WHITE_RATING_COLUMN, BLACK_RATING_COLUMN)
        .profile(LICHESS_PROFILE)
        .display_name(LICHESS_DISPLAY_NAME)
        .evidence_key(GAME_ID_COLUMN)
        .prepare(destination)?;

    if !launch {
        println!("Prepared {}", prepared.manifest_path().display());
        return Ok(());
    }
    let mut workbench = prepared.launch()?;
    let status = workbench.wait()?;
    if status.success() {
        return Ok(());
    }
    Err(format!("RawScope workbench exited with {status}").into())
}
