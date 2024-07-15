pub mod mutable_modules;
use dashmap::mapref::one::{Ref, RefMut};
use module_memory_store::ModuleMemoryStore;
use mutable_modules::MutableModulesMemoryStore;
pub mod immutable_modules;
use immutable_modules::ImmutableModulesMemoryStore;
pub mod module_memory_store;
use toy_farm_macro_cache_item::cache_item;

use crate::{Module, ModuleGraphEdge, ModuleId};

pub struct ModuleCacheManager {
    /// Store is responsible for how to read and load cache from disk.
    pub mutable_modules_store: MutableModulesMemoryStore,
    pub immutable_modules_store: ImmutableModulesMemoryStore,
}

#[cache_item]
#[derive(Debug, Clone, PartialEq)]
pub struct CachedModuleDependency {
    pub dependency: ModuleId,
    pub edge_info: ModuleGraphEdge,
}

#[cache_item]
#[derive(Debug, Clone, PartialEq)]
pub struct CachedWatchDependency {
    pub dependency: ModuleId,
    pub timestamp: u128,
    pub hash: String,
}
#[derive(Clone)]
#[cache_item]
pub struct CachedModule {
    pub module: Module,
    pub dependencies: Vec<CachedModuleDependency>,
    pub watch_dependencies: Vec<CachedWatchDependency>,
}

impl ModuleCacheManager {
    pub fn new(cache_dir: &str, namespace: &str, mode: crate::Mode) -> Self {
        Self {
            mutable_modules_store: MutableModulesMemoryStore::new(
                cache_dir,
                namespace,
                mode.clone(),
            ),
            immutable_modules_store: ImmutableModulesMemoryStore::new(
                cache_dir,
                namespace,
                mode.clone(),
            ),
        }
    }

    pub fn is_cache_changed(&self, module: &Module) -> bool {
        if module.immutable {
            self.immutable_modules_store.is_cache_changed(module)
        } else {
            self.mutable_modules_store.is_cache_changed(module)
        }
    }

    pub fn has_cache(&self, key: &ModuleId) -> bool {
        self.mutable_modules_store.has_cache(key) || self.immutable_modules_store.has_cache(key)
    }

    pub fn set_cache(&self, key: ModuleId, module: CachedModule) {
        if module.module.immutable {
            self.immutable_modules_store.set_cache(key, module);
        } else {
            self.mutable_modules_store.set_cache(key, module);
        }
    }

    pub fn get_cache(&self, key: &ModuleId) -> CachedModule {
        if let Some(module) = self.mutable_modules_store.get_cache(key) {
            return module;
        }

        self.immutable_modules_store
            .get_cache(key)
            .expect("Cache broken, please remove node_modules/.farm and retry.")
    }

    pub fn get_cache_ref(&self, key: &ModuleId) -> Ref<'_, ModuleId, CachedModule> {
        if let Some(module) = self.mutable_modules_store.get_cache_ref(key) {
            return module;
        }

        self.immutable_modules_store
            .get_cache_ref(key)
            .expect("Cache broken, please remove node_modules/.farm and retry.")
    }

    pub fn get_cache_mut_ref(&self, key: &ModuleId) -> RefMut<'_, ModuleId, CachedModule> {
        if let Some(module) = self.mutable_modules_store.get_cache_mut_ref(key) {
            return module;
        }

        return self
            .immutable_modules_store
            .get_cache_mut_ref(key)
            .expect("Cache broken, please remove node_modules/.farm and retry.");
    }

    pub async fn write_cache(&self) {
        tokio::join!(
            self.mutable_modules_store.write_cache(),
            self.immutable_modules_store.write_cache()
        );
    }

    pub fn invalidate_cache(&self, key: &ModuleId) {
        self.mutable_modules_store.invalidate_cache(key);
        self.immutable_modules_store.invalidate_cache(key);
    }

    pub fn cache_outdated(&self, key: &ModuleId) -> bool {
        self.mutable_modules_store.cache_outdated(key)
            || self.immutable_modules_store.cache_outdated(key)
    }
}
