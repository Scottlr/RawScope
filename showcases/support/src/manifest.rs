//! Compile-time dataset metadata shown before acquisition is enabled.

/// Upstream identity, attribution, and planned inputs for one showcase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatasetManifest {
    pub id: &'static str,
    pub name: &'static str,
    pub source_url: &'static str,
    pub repository_url: &'static str,
    pub license_name: &'static str,
    pub license_url: &'static str,
    pub citation: Option<&'static str>,
    pub revision: Option<&'static str>,
    pub checksum: Option<&'static str>,
    pub planned_inputs: &'static [&'static str],
    pub planned_uses: &'static [&'static str],
    pub caveats: &'static [&'static str],
    pub implementation_status: &'static str,
}
