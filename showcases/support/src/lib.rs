//! Shared contracts for scaffolded RawScope dataset showcases.

mod command;
mod dataset;
mod error;
mod manifest;
mod paths;

pub use command::{run_showcase, ShowcaseCommand, SHOWCASE_COMMAND_USAGE};
pub use dataset::DatasetShowcase;
pub use error::{not_implemented, Result, ShowcaseError};
pub use manifest::DatasetManifest;
pub use paths::{ShowcaseContext, ShowcasePaths, SHOWCASE_DATA_DIRECTORY};
