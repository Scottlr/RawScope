//! Future NASA SMAP/MSL RawScope launch boundary.

use rawscope_showcase_support::{not_implemented, Result, ShowcaseContext};

pub fn visualise(_context: &ShowcaseContext) -> Result<()> {
    Err(not_implemented("nasa-smap-msl", "visualise"))
}
