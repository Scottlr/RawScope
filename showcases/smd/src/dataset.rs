//! SMD metadata and lifecycle routing.

use rawscope_showcase_support::{DatasetManifest, DatasetShowcase, Result, ShowcaseContext};

use crate::{analyse, fetch, transform, visualise};

const SMD_ID: &str = "smd";

const SMD_MANIFEST: DatasetManifest = DatasetManifest {
    id: SMD_ID,
    name: "Server Machine Dataset (SMD)",
    source_url: "https://github.com/NetManAIOps/OmniAnomaly/tree/master/ServerMachineDataset",
    repository_url: "https://github.com/NetManAIOps/OmniAnomaly",
    license_name: "Expected MIT; verify upstream dataset terms",
    license_url: "https://github.com/NetManAIOps/OmniAnomaly/blob/master/LICENSE",
    citation: Some(
        "Su et al., Robust Anomaly Detection for Multivariate Time Series through Stochastic Recurrent Neural Network (KDD 2019)",
    ),
    revision: None,
    checksum: None,
    planned_inputs: &[
        "per-machine training sequences",
        "per-machine test sequences",
        "point anomaly labels",
        "interpretation labels identifying contributing dimensions",
    ],
    planned_uses: &[
        "dimension-level anomaly windows",
        "machine incident construction",
        "affected-dimension concurrency",
        "contributing-dimension coverage",
        "machine and group roll-ups",
    ],
    caveats: &[
        "Fleet-wide wall-clock alignment must be verified before synchronized machine aggregation is claimed.",
        "Upstream licence and dataset terms must be reviewed at the pinned revision.",
    ],
    implementation_status: "scaffold only; no downloads or processing have been performed",
};

pub struct SmdShowcase;

impl DatasetShowcase for SmdShowcase {
    fn manifest(&self) -> &'static DatasetManifest {
        &SMD_MANIFEST
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
