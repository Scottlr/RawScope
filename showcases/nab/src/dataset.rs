//! NAB metadata and lifecycle routing.

use rawscope_showcase_support::{DatasetManifest, DatasetShowcase, Result, ShowcaseContext};

use crate::{analyse, fetch, transform, visualise};

const NAB_ID: &str = "nab";

const NAB_MANIFEST: DatasetManifest = DatasetManifest {
    id: NAB_ID,
    name: "Numenta Anomaly Benchmark (NAB)",
    source_url: "https://github.com/numenta/NAB/tree/master/data",
    repository_url: "https://github.com/numenta/NAB",
    license_name: "Expected AGPL-3.0; verify upstream dataset terms",
    license_url: "https://github.com/numenta/NAB/blob/master/LICENSE.txt",
    citation: Some(
        "Lavin and Ahmad, Evaluating Real-Time Anomaly Detection Algorithms: The Numenta Anomaly Benchmark (2015)",
    ),
    revision: None,
    checksum: None,
    planned_inputs: &[
        "timestamped metric CSV files",
        "labelled anomaly windows",
        "detector result files",
    ],
    planned_uses: &[
        "ground-truth versus detector windows",
        "overlap, residual, missing, and lead/lag",
        "detector consensus",
        "corpus-level detector roll-ups",
    ],
    caveats: &["Upstream licence and dataset terms must be reviewed at the pinned revision."],
    implementation_status: "scaffold only; no downloads or processing have been performed",
};

pub struct NabShowcase;

impl DatasetShowcase for NabShowcase {
    fn manifest(&self) -> &'static DatasetManifest {
        &NAB_MANIFEST
    }

    fn fetch(&self, context: &ShowcaseContext) -> Result<()> {
        fetch::fetch(context)
    }

    fn transform(&self, context: &ShowcaseContext) -> Result<()> {
        transform::transform(context)
    }

    fn analyse(&self, context: &ShowcaseContext) -> Result<()> {
        analyse::analyse(context)
    }

    fn visualise(&self, context: &ShowcaseContext) -> Result<()> {
        visualise::visualise(context)
    }
}
