pub mod mutable_modules;
use mutable_modules::MutableModulesMemoryStore;
pub mod immutable_modules;
use immutable_modules::ImmutableModulesMemoryStore;

use crate::{Module, ModuleGraphEdge, ModuleId};

pub struct ModuleCacheManager {
    /// Store is responsible for how to read and load cache from disk.
    pub mutable_modules_store: MutableModulesMemoryStore,
    pub immutable_modules_store: ImmutableModulesMemoryStore,
}

#[derive(Debug, Clone)]
pub struct CachedModuleDependency {
    pub dependency: ModuleId,
    pub edge_info: ModuleGraphEdge,
}

#[derive(Debug, Clone)]
pub struct CachedWatchDependency {
    pub dependency: ModuleId,
    pub timestamp: u128,
    pub hash: String,
}
#[derive(Clone)]
pub struct CachedModule {
    pub module: Module,
    pub dependencies: Vec<CachedModuleDependency>,
    pub watch_dependencies: Vec<CachedWatchDependency>,
}
