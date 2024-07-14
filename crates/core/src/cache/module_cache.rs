pub mod mutable_modules;
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
#[derive(Clone, Debug, PartialEq)]
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
}
