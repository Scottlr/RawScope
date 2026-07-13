//! Optional source adapters and transform helpers for RawScope.
//!
//! Each integration owns its source-specific transformation in a feature-gated
//! module. All integrations converge on a normal session-v1 bundle and the same
//! [`PreparedAdapterSession`] launch contract.

use std::path::Path;

mod prepared_session;

pub use prepared_session::PreparedAdapterSession;
pub use rawscope_session::{SessionManifestError, WorkbenchLaunchError, WorkbenchLauncher};

#[cfg(feature = "spanfold")]
pub mod spanfold;

/// Common boundary for integrations that prepare persistent RawScope sessions.
///
/// The source and error remain adapter-owned so integrations do not have to
/// erase useful domain types or error context.
pub trait SessionAdapter {
    /// Source value consumed by this adapter.
    type Input: ?Sized;

    /// Adapter-specific preparation failure.
    type Error;

    /// Transforms `input` and writes a persistent session bundle at `destination`.
    fn prepare_session(
        &self,
        input: &Self::Input,
        destination: &Path,
    ) -> Result<PreparedAdapterSession, Self::Error>;
}
