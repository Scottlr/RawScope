//! CLI entry point for the scaffolded SMD showcase.

mod analyse;
mod dataset;
mod fetch;
mod transform;
mod visualise;

use rawscope_showcase_support::{run_showcase, Result};

fn main() -> Result<()> {
    run_showcase(
        &dataset::SmdShowcase,
        env!("CARGO_MANIFEST_DIR"),
        std::env::args_os().skip(1),
    )
}
