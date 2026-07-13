//! Shared command parsing and dispatch for showcase binaries.

use std::ffi::OsString;

use crate::{DatasetShowcase, Result, ShowcaseContext, ShowcaseError};

/// Command synopsis shared by all showcase binaries.
pub const SHOWCASE_COMMAND_USAGE: &str = "info|fetch|transform|analyse|visualise|run";

/// Supported showcase commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShowcaseCommand {
    Info,
    Fetch,
    Transform,
    Analyse,
    Visualise,
    Run,
}

impl ShowcaseCommand {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let mut args = args.into_iter();
        let supplied = args.next().map(|arg| arg.to_string_lossy().into_owned());
        if args.next().is_some() {
            return Err(ShowcaseError::InvalidCommand { supplied });
        }

        match supplied.as_deref() {
            Some("info") => Ok(Self::Info),
            Some("fetch") => Ok(Self::Fetch),
            Some("transform") => Ok(Self::Transform),
            Some("analyse") => Ok(Self::Analyse),
            Some("visualise") => Ok(Self::Visualise),
            Some("run") => Ok(Self::Run),
            _ => Err(ShowcaseError::InvalidCommand { supplied }),
        }
    }
}

/// Parses and dispatches one showcase command.
pub fn run_showcase(
    showcase: &impl DatasetShowcase,
    manifest_dir: impl AsRef<std::path::Path>,
    args: impl IntoIterator<Item = OsString>,
) -> Result<()> {
    let command = ShowcaseCommand::parse(args)?;
    let manifest = showcase.manifest();
    let context = ShowcaseContext::from_manifest_dir(manifest_dir, manifest.id)?;

    match command {
        ShowcaseCommand::Info => {
            print_info(manifest, &context);
            Ok(())
        }
        ShowcaseCommand::Fetch => showcase.fetch(&context),
        ShowcaseCommand::Transform => showcase.transform(&context),
        ShowcaseCommand::Analyse => showcase.analyse(&context),
        ShowcaseCommand::Visualise => showcase.visualise(&context),
        ShowcaseCommand::Run => showcase.run(&context),
    }
}

fn print_info(manifest: &crate::DatasetManifest, context: &ShowcaseContext) {
    println!("Dataset: {}", manifest.name);
    println!("Source repository: {}", manifest.repository_url);
    println!(
        "Expected licence: {} ({})",
        manifest.license_name, manifest.license_url
    );
    println!("Citation: {}", manifest.citation.unwrap_or("not recorded"));
    println!(
        "Pinned revision: {}",
        manifest.revision.unwrap_or("not pinned")
    );
    println!("Checksum: {}", manifest.checksum.unwrap_or("not recorded"));
    println!("Planned local paths:");
    println!("  downloads: {}", context.paths.downloads.display());
    println!("  extracted: {}", context.paths.extracted.display());
    println!("  transformed: {}", context.paths.transformed.display());
    println!("  analysis: {}", context.paths.analysis.display());
    println!("  cache: {}", context.paths.cache.display());
    println!("Current status: {}", manifest.implementation_status);
}
