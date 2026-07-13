//! Dataset-specific lifecycle boundary shared by showcase binaries.

use crate::{not_implemented, DatasetManifest, Result, ShowcaseContext};

/// Operations implemented independently by each dataset showcase.
pub trait DatasetShowcase {
    fn manifest(&self) -> &'static DatasetManifest;

    fn fetch(&self, context: &ShowcaseContext) -> Result<()>;

    fn transform(&self, context: &ShowcaseContext) -> Result<()>;

    fn analyse(&self, context: &ShowcaseContext) -> Result<()>;

    fn visualise(&self, context: &ShowcaseContext) -> Result<()>;

    fn run(&self, _context: &ShowcaseContext) -> Result<()> {
        Err(not_implemented(self.manifest().id, "run"))
    }
}
