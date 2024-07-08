use dashmap::DashMap;

use crate::{cache_store::CacheStore, ModuleId};

use super::CachedModule;

#[allow(dead_code)]
pub struct MutableModulesMemoryStore {
    /// low level cache store
    store: CacheStore,
    /// ModuleId -> Cached Module
    cached_modules: DashMap<ModuleId, CachedModule>,
}
