//! CLI entry point for the scaffolded NAB showcase.

mod analyse;
mod dataset;
mod fetch;
mod transform;
mod visualise;

use rawscope_showcase_support::{run_showcase, Result};

fn main() -> Result<()> {
    run_showcase(
        &dataset::NabShowcase,
        env!("CARGO_MANIFEST_DIR"),
        std::env::args_os().skip(1),
    )
}
