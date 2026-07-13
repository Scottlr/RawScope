//! Dataset-specific lifecycle boundary shared by showcase binaries.

use crate::{DatasetManifest, Result, ShowcaseContext};

/// Operations implemented independently by each dataset showcase.
pub trait DatasetShowcase {
    fn manifest(&self) -> &'static DatasetManifest;

    fn fetch(&self, context: &ShowcaseContext) -> Result<()>;

    fn transform(&self, context: &ShowcaseContext) -> Result<()>;

    fn analyse(&self, context: &ShowcaseContext) -> Result<()>;

    fn visualise(&self, context: &ShowcaseContext) -> Result<()>;
}
