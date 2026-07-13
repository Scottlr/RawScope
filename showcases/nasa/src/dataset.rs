//! NASA SMAP/MSL metadata and lifecycle routing.

use rawscope_showcase_support::{DatasetManifest, DatasetShowcase, Result, ShowcaseContext};

use crate::{analyse, fetch, transform, visualise};

const NASA_ID: &str = "nasa-smap-msl";

const NASA_MANIFEST: DatasetManifest = DatasetManifest {
    id: NASA_ID,
    name: "NASA SMAP/MSL telemetry anomaly dataset",
    source_url: "https://github.com/khundman/telemanom/tree/master/data",
    repository_url: "https://github.com/khundman/telemanom",
    license_name: "Expected Apache-2.0; verify telemetry and attribution terms",
    license_url: "https://github.com/khundman/telemanom/blob/master/LICENSE",
    citation: Some(
        "Hundman et al., Detecting Spacecraft Anomalies Using LSTMs and Nonparametric Dynamic Thresholding (KDD 2018)",
    ),
    revision: None,
    checksum: None,
    planned_inputs: &[
        "train and test telemetry arrays",
        "channel metadata",
        "labelled anomaly sequences",
        "anomaly classes",
    ],
    planned_uses: &[
        "channel-level anomaly windows",
        "detector comparison",
        "spacecraft roll-ups",
        "channel-family aggregation",
        "point versus contextual anomaly summaries",
    ],
    caveats: &[
        "Cross-channel time alignment must be verified before synchronized incident aggregation is claimed.",
        "Upstream licence, NASA attribution, and dataset terms must be reviewed at the pinned revision.",
    ],
    implementation_status: "scaffold only; no downloads or processing have been performed",
};

pub struct NasaShowcase;

impl DatasetShowcase for NasaShowcase {
    fn manifest(&self) -> &'static DatasetManifest {
        &NASA_MANIFEST
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
