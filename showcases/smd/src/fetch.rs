//! Future SMD acquisition boundary.

use rawscope_showcase_support::{not_implemented, Result, ShowcaseContext};

pub fn fetch(_context: &ShowcaseContext) -> Result<()> {
    Err(not_implemented("smd", "fetch"))
}
