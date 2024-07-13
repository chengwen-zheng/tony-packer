use cache_store::CacheStore;
use module_cache::ModuleCacheManager;
use tokio::sync::Mutex;

use crate::Mode;

pub mod cache_store;
pub mod module_cache;

pub struct CacheManager {
    pub module_cache: ModuleCacheManager,

    pub lazy_compile_store: CacheStore,

    pub custom: CacheStore,

    pub lock: Mutex<bool>,
}

impl CacheManager {
    pub fn new(cache_dir: &str, namespace: &str, mode: Mode) -> Self {
        Self {
            module_cache: ModuleCacheManager::new(cache_dir, namespace, mode.clone()),
            lazy_compile_store: CacheStore::new(cache_dir, namespace, mode.clone(), "lazy-compile"),
            custom: CacheStore::new(cache_dir, namespace, mode.clone(), "custom"),
            lock: Mutex::new(false),
        }
    }

    pub async fn write_cache(&self) {
        let mut lock = self.lock.lock().await;
        *lock = true;

        // TODO: write cache

        *lock = false;
    }
}
