use std::collections::{HashMap, HashSet};

use dashmap::DashMap;
use toy_farm_macro_cache_item::cache_item;
use toy_farm_utils::hash::sha256;

use crate::{
    cache_store::{CacheStore, CacheStoreKey},
    utils::cache_panic,
    Mode, ModuleId,
};

use super::{module_memory_store::ModuleMemoryStore, CachedModule};

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

#[cache_item]
pub struct CachedPackage {
    pub list: Vec<CachedModule>,
    name: String,
    version: String,
}
impl CachedPackage {
    pub fn gen_key(name: &str, version: &str) -> String {
        format!("{}@{}", name, version)
    }

    pub fn key(&self) -> String {
        Self::gen_key(&self.name, &self.version)
    }
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

    pub fn read_cached_package(&self, package_key: &str) -> CachedPackage {
        let cache = self
            .store
            .read_cache(package_key)
            .expect("Cache broken, please remove node_modules/.farm and retry.");

        crate::deserialize!(&cache, CachedPackage)
    }

    pub fn read_package(&self, module_id: &ModuleId) -> Option<()> {
        if let Some(package_key) = self.manifest.get(module_id) {
            let package = self.read_cached_package(package_key.value());

            for module in package.list {
                self.cached_modules.insert(module.module.id.clone(), module);
            }

            return Some(());
        }

        None
    }
}

impl ModuleMemoryStore for ImmutableModulesMemoryStore {
    fn is_cache_changed(&self, module: &crate::Module) -> bool {
        // we do not need to check the hash of immutable modules, just check the cache
        !self.has_cache(&module.id)
    }

    fn has_cache(&self, key: &ModuleId) -> bool {
        if self.cached_modules.contains_key(key) {
            return true;
        }

        if let Some(package_key) = self.manifest.get(key) {
            return self.store.has_cache(package_key.value());
        }

        false
    }

    fn set_cache(&self, key: ModuleId, module: CachedModule) {
        self.cached_modules.insert(key, module);
    }

    fn get_cache(&self, key: &ModuleId) -> Option<CachedModule> {
        if let Some((_, module)) = self.cached_modules.remove(key) {
            return Some(module);
        }

        if let Some(package_key) = self.manifest.get(key) {
            let package = self.read_cached_package(package_key.value());

            for module in package.list {
                self.cached_modules.insert(module.module.id.clone(), module);
            }

            return self.cached_modules.remove(key).map(|(_, module)| module);
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

        if let Some(package_key) = self.manifest.get(key) {
            let package = self.read_cached_package(package_key.value());

            for module in package.list {
                self.cached_modules.insert(module.module.id.clone(), module);
            }

            return self.cached_modules.get(key);
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

        if let Some(package_key) = self.manifest.get(key) {
            let package = self.read_cached_package(package_key.value());

            for module in package.list {
                self.cached_modules.insert(module.module.id.clone(), module);
            }

            return self.cached_modules.get_mut(key);
        }

        None
    }

    fn invalidate_cache(&self, key: &ModuleId) {
        self.cached_modules.remove(key);
    }

    fn cache_outdated(&self, key: &ModuleId) -> bool {
        if let Some(package_key) = self.manifest.get(key) {
            return !self.store.has_cache(package_key.value());
        }

        false
    }

    async fn write_cache(&self) {
        let mut packages = HashMap::new();

        // group modules by package
        for item in self.cached_modules.iter() {
            let module = item.value();
            let package_key =
                CachedPackage::gen_key(&module.module.package_name, &module.module.package_version);

            let package = packages.entry(package_key.clone()).or_insert_with(Vec::new);

            package.push(item.key().clone());
            self.manifest.insert(item.key().clone(), package_key);
        }

        // write packages
        let manifest = self
            .manifest
            .iter()
            .map(|item| (item.key().to_string(), item.value().to_string()))
            .collect::<HashMap<String, String>>();

        let manifest_bytes = serde_json::to_vec(&manifest)
            .unwrap_or_else(|e| cache_panic(&e.to_string(), &self.cache_dir));

        let mut tasks = vec![];
        for (package_key, module_ids) in packages {
            let store = self.store.clone();
            let cache_dir = self.cache_dir.clone();
            let cached_modules = self.cached_modules.clone();
            let manifest_reversed = self.manifest_reversed.clone();
            let task = tokio::spawn(async move {
                let gen_cache_store_key = |mut modules: Vec<String>| {
                    modules.sort();

                    CacheStoreKey {
                        name: package_key.clone(),
                        key: sha256(modules.join(",").as_bytes(), 32),
                    }
                };

                let read_cached_package = |package_key: &str| -> CachedPackage {
                    let cache = store
                        .read_cache(package_key)
                        .unwrap_or_else(|| cache_panic(package_key, &cache_dir));

                    crate::deserialize!(&cache, CachedPackage)
                };

                // the package is already cached, we only need to update it

                if manifest_reversed.contains_key(&package_key) {
                    let modules_in_package = manifest_reversed.get(&package_key).unwrap();

                    let add_modules = module_ids
                        .iter()
                        .filter(|module_id| !modules_in_package.contains(module_id))
                        .collect::<Vec<_>>();

                    // add new modules to the package
                    if !add_modules.is_empty() {
                        let mut package = read_cached_package(&package_key);

                        package.list.extend(
                            add_modules
                                .into_iter()
                                .map(|module_id| {
                                    cached_modules
                                        .get(module_id)
                                        .unwrap_or_else(|| cache_panic(&package_key, &cache_dir))
                                        .clone()
                                })
                                .collect::<Vec<_>>(),
                        );

                        let modules = package
                            .list
                            .iter()
                            .map(|cm| cm.module.id.to_string())
                            .collect::<Vec<_>>();
                        let package_bytes = crate::serialize!(&package);

                        let store_key = gen_cache_store_key(modules);
                        let _ = store.write_single_cache(store_key, package_bytes).await;
                    }
                }

                let module_strings = module_ids.iter().map(|m| m.to_string()).collect::<Vec<_>>();
                let package = CachedPackage {
                    list: module_ids
                        .iter()
                        .map(|module_id| {
                            cached_modules
                                .get(module_id)
                                .unwrap_or_else(|| cache_panic(&package_key, &cache_dir))
                                .clone()
                        })
                        .collect(),
                    name: package_key.split('@').next().unwrap().to_string(),
                    version: package_key.split('@').last().unwrap().to_string(),
                };

                let package_bytes = crate::serialize!(&package);
                let store_key = gen_cache_store_key(module_strings);
                let _ = store.write_single_cache(store_key, package_bytes).await;
            });

            tasks.push(task);
        }

        let mut cache_map = HashMap::new();
        cache_map.insert(
            CacheStoreKey {
                name: MANIFEST_KEY.to_string(),
                key: sha256(manifest_bytes.as_slice(), 32),
            },
            manifest_bytes,
        );

        self.store.write_cache(cache_map).await;
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
        let namespace = "test_immutable_modules_memory_store";
        let mode = Mode::Development;

        let store = ImmutableModulesMemoryStore::new(cache_dir, namespace, mode);

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

        // assert_eq!(store.cache_outdated(&module.id), true);

        store.write_cache().await;
    }
}
