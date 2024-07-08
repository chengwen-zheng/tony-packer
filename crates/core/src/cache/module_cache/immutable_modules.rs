use std::collections::HashSet;

use dashmap::DashMap;

use crate::{cache_store::CacheStore, ModuleId};

use super::CachedModule;

#[allow(dead_code)]
pub struct ImmutableModulesMemoryStore {
    cache_dir: String,
    /// low level cache store
    store: CacheStore,
    /// ModuleId -> Cached Module
    cached_modules: DashMap<ModuleId, CachedModule>,
    /// moduleId -> PackageKey
    manifest: DashMap<ModuleId, String>,
    manifest_reversed: DashMap<String, HashSet<ModuleId>>,
}
