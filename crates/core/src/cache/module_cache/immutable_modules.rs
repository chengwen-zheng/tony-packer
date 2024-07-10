use std::collections::{HashMap, HashSet};

use dashmap::DashMap;

use crate::{cache_store::CacheStore, Mode, ModuleId};

use super::CachedModule;

const MANIFEST_KEY: &str = "immutable-modules.json";

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

impl ImmutableModulesMemoryStore {
    pub fn new(cache_dir_str: &str, namespace: &str, mode: Mode) -> Self {
        let store = CacheStore::new(cache_dir_str, namespace, mode, "immutable-modules");
        let manifest_bytes: Vec<u8> = store.read_cache(MANIFEST_KEY).unwrap_or_default();

        let manifest: HashMap<String, String> =
            serde_json::from_slice(&manifest_bytes).unwrap_or_default();
        let manifest = manifest
            .into_iter()
            .map(|(key, value)| (ModuleId::from(key), value))
            .collect::<HashMap<ModuleId, String>>();

        let manifest_reversed = DashMap::new();

        for (key, value) in manifest.iter() {
            let mut set = manifest_reversed
                .entry(value.clone())
                .or_insert_with(HashSet::new);
            set.insert(key.clone());
        }

        Self {
            cache_dir: cache_dir_str.to_string(),
            store,
            cached_modules: DashMap::new(),
            manifest: manifest.into_iter().collect(),
            manifest_reversed,
        }
    }
}
