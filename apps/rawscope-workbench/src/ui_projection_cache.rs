//! Revision-keyed immutable UI projections for the workbench facade.

use std::sync::Arc;

use crate::active_generation::WorkbenchGenerationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct UiProjectionKey {
    pub(crate) active_generation: WorkbenchGenerationId,
    pub(crate) view_revision: u64,
    pub(crate) operational_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiProjection {
    pub(crate) title: Arc<str>,
    pub(crate) status: Arc<str>,
}

#[derive(Debug, Default)]
pub(crate) struct UiProjectionCache {
    entry: Option<(UiProjectionKey, Arc<UiProjection>)>,
}

impl UiProjectionCache {
    pub(crate) fn get_or_build(
        &mut self,
        key: UiProjectionKey,
        build: impl FnOnce() -> UiProjection,
    ) -> Arc<UiProjection> {
        if let Some((cached_key, projection)) = &self.entry {
            if *cached_key == key {
                return Arc::clone(projection);
            }
        }

        let projection = Arc::new(build());
        self.entry = Some((key, Arc::clone(&projection)));
        projection
    }

    pub(crate) fn invalidate(&mut self) {
        self.entry = None;
    }

    pub(crate) fn key(&self) -> Option<UiProjectionKey> {
        self.entry.as_ref().map(|(key, _)| *key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(view_revision: u64) -> UiProjectionKey {
        UiProjectionKey {
            active_generation: WorkbenchGenerationId::new(7),
            view_revision,
            operational_revision: 3,
        }
    }

    fn projection(status: &str) -> UiProjection {
        UiProjection {
            title: Arc::from("RawScope"),
            status: Arc::from(status),
        }
    }

    #[test]
    fn same_revision_reuses_the_immutable_projection_arc() {
        let mut cache = UiProjectionCache::default();
        let first = cache.get_or_build(key(1), || projection("ready"));
        let second = cache.get_or_build(key(1), || projection("must not rebuild"));

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(second.status.as_ref(), "ready");
        assert_eq!(cache.key(), Some(key(1)));
    }

    #[test]
    fn changed_revision_replaces_only_the_projection_entry() {
        let mut cache = UiProjectionCache::default();
        let first = cache.get_or_build(key(1), || projection("ready"));
        let second = cache.get_or_build(key(2), || projection("settling"));

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(second.status.as_ref(), "settling");
        assert_eq!(cache.key(), Some(key(2)));
    }

    #[test]
    fn explicit_invalidation_drops_obsolete_shared_state() {
        let mut cache = UiProjectionCache::default();
        let _ = cache.get_or_build(key(1), || projection("ready"));
        cache.invalidate();

        assert_eq!(cache.key(), None);
    }
}
