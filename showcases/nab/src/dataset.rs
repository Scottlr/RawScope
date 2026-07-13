//! NAB metadata and lifecycle routing.

use rawscope_showcase_support::{DatasetManifest, DatasetShowcase, Result, ShowcaseContext};

use crate::{analyse, fetch, transform, visualise};

const NAB_ID: &str = "nab";

const NAB_MANIFEST: DatasetManifest = DatasetManifest {
    id: NAB_ID,
    name: "Numenta Anomaly Benchmark (NAB)",
    source_url: "https://raw.githubusercontent.com/numenta/NAB/ea702d75cc2258d9d7dd35ca8e5e2539d71f3140/data/realTraffic/speed_7578.csv",
    repository_url: "https://github.com/numenta/NAB",
    license_name: "MIT",
    license_url: "https://github.com/numenta/NAB/blob/ea702d75cc2258d9d7dd35ca8e5e2539d71f3140/LICENSE.txt",
    citation: Some(
        "Lavin and Ahmad, Evaluating Real-Time Anomaly Detection Algorithms: The Numenta Anomaly Benchmark (2015)",
    ),
    revision: Some("ea702d75cc2258d9d7dd35ca8e5e2539d71f3140"),
    checksum: Some("sha256:15ba994384730cb5a2e6818f17ded1430b40d92eb7a08e5b1a48a445c8c350f6"),
    planned_inputs: &["realTraffic/speed_7578.csv (1,127 traffic-speed observations)"],
    planned_uses: &[
        "traffic-speed value shape over source order",
        "timestamp-preserving row evidence",
        "small external-API integration example",
    ],
    caveats: &[
        "This minimal showcase uses one series, not the full NAB benchmark corpus.",
        "It visualises source values and does not calculate NAB detector scores.",
    ],
    implementation_status: "fetch, transform, visualise, and run are implemented for the smallest NAB series",
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

    fn run(&self, context: &ShowcaseContext) -> Result<()> {
        fetch::fetch(context)?;
        transform::transform(context)?;
        visualise::visualise(context)
    }
}
