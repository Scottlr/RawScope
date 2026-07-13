//! CLI entry point for the scaffolded NASA SMAP/MSL showcase.

mod analyse;
mod dataset;
mod fetch;
mod transform;
mod visualise;

use rawscope_showcase_support::{run_showcase, Result};

fn main() -> Result<()> {
    run_showcase(
        &dataset::NasaShowcase,
        env!("CARGO_MANIFEST_DIR"),
        std::env::args_os().skip(1),
    )
}
