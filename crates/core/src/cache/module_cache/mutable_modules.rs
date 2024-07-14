use std::collections::HashMap;

use dashmap::DashMap;
use toy_farm_utils::hash::sha256;

use crate::cache_store::CacheStoreKey;
use crate::{cache_store::CacheStore, Mode, Module, ModuleId};
use crate::{deserialize, serialize};

use super::module_memory_store::ModuleMemoryStore;
use super::CachedModule;
use rkyv::Deserialize;

#[allow(dead_code)]
pub struct MutableModulesMemoryStore {
    /// low level cache store
    store: CacheStore,
    /// ModuleId -> Cached Module
    cached_modules: DashMap<ModuleId, CachedModule>,
}

impl MutableModulesMemoryStore {
    pub fn new(cache_dir_str: &str, namespace: &str, mode: Mode) -> Self {
        Self {
            store: CacheStore::new(cache_dir_str, namespace, mode.clone(), "mutable-modules"),
            cached_modules: DashMap::new(),
        }
    }

    fn gen_cache_store_key(&self, module: &crate::module::Module) -> CacheStoreKey {
        let hash_key = sha256(
            format!("{}{}", module.content_hash, module.id).as_bytes(),
            32,
        );
        CacheStoreKey {
            name: module.id.to_string(),
            key: hash_key,
        }
    }
}

impl ModuleMemoryStore for MutableModulesMemoryStore {
    fn is_cache_changed(&self, module: &Module) -> bool {
        let store_key = self.gen_cache_store_key(module);
        self.store.is_cache_changed(&store_key)
    }

    fn has_cache(&self, key: &ModuleId) -> bool {
        self.cached_modules.contains_key(key)
    }

    fn set_cache(&self, key: ModuleId, module: CachedModule) {
        self.cached_modules.insert(key, module);
    }

    fn get_cache(&self, key: &ModuleId) -> Option<CachedModule> {
        // fist get cache from memory
        if let Some((_, module)) = self.cached_modules.remove(key) {
            return Some(module);
        }

        // then get cache from disk
        let cache = self.store.read_cache(&key.to_string());
        if let Some(cache) = cache {
            let module = deserialize!(&cache, CachedModule);
            return Some(module);
        }

        None
    }

    fn get_cache_ref(
        &self,
        key: &ModuleId,
    ) -> Option<dashmap::mapref::one::Ref<'_, ModuleId, CachedModule>> {
        if let Some(module) = self.cached_modules.get(key) {
            return Some(module);
        }

        let cache = self.store.read_cache(&key.to_string());

        if let Some(cache) = cache {
            let module = deserialize!(&cache, CachedModule);
            self.cached_modules.insert(key.clone(), module);
            return Some(self.cached_modules.get(key).unwrap());
        }

        None
    }

    fn get_cache_mut_ref(
        &self,
        key: &ModuleId,
    ) -> Option<dashmap::mapref::one::RefMut<'_, ModuleId, CachedModule>> {
        if let Some(module) = self.cached_modules.get_mut(key) {
            return Some(module);
        }

        let cache = self.store.read_cache(&key.to_string());
        if let Some(cache) = cache {
            let module = deserialize!(&cache, CachedModule);
            self.cached_modules.insert(key.clone(), module);
            return Some(self.cached_modules.get_mut(key).unwrap());
        }

        None
    }

    fn invalidate_cache(&self, key: &ModuleId) {
        self.cached_modules.remove(key);
    }

    fn cache_outdated(&self, key: &ModuleId) -> bool {
        !self.cached_modules.contains_key(key)
    }

    async fn write_cache(&self) {
        let mut cache_map = HashMap::new();

        for entry in self.cached_modules.iter() {
            let module = entry.value();
            let store_key = self.gen_cache_store_key(&module.module);

            if self.store.is_cache_changed(&store_key) {
                cache_map.insert(store_key, module.clone());
            }
        }

        let futures = cache_map
            .iter()
            .map(|(store_key, module)| {
                let bytes = serialize!(module);
                self.store.write_single_cache(store_key.clone(), bytes)
            })
            .collect::<Vec<_>>();

        futures::future::join_all(futures).await;

        self.store.write_manifest().await;
    }
}

// MARK: - Tests
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{Module, ModuleType};
    use toy_farm_macro_cache_item::cache_item;

    #[cache_item]
    #[derive(Debug, Clone)]
    struct TestModule {
        id: String,
    }

    #[tokio::test]
    async fn test_immutable_modules_memory_store() {
        let cache_dir = "./cache";
        let namespace = "test_mutable_modules_memory_store";
        let mode = Mode::Development;

        let store = MutableModulesMemoryStore::new(cache_dir, namespace, mode);

        let module = Module {
            id: ModuleId::from("test"),
            package_name: "test".to_string(),
            package_version: "0.2.0".to_string(),
            content_hash: "test".to_string(),
            side_effects: true,
            source_map_chain: vec![],
            external: false,
            immutable: false,
            execution_order: 0,
            size: 0,
            used_exports: vec![],
            last_update_timestamp: 0,
            content: Arc::new("".to_string()),
            module_type: ModuleType::Custom("__farm_unknown".to_string()),
        };

        let cached_module = CachedModule {
            module: module.clone(),
            dependencies: vec![],
            watch_dependencies: vec![],
        };

        store.set_cache(module.id.clone(), cached_module.clone());

        // read cache from memory
        let cached_module = store.get_cache(&module.id).unwrap();
        assert_eq!(cached_module, cached_module.clone());

        store.write_cache().await;
    }
}
