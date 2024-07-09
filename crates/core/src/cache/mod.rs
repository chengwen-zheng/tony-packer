use cache_store::CacheStore;
use module_cache::ModuleCacheManager;
use tokio::sync::Mutex;

pub mod cache_store;
pub mod module_cache;

pub struct CacheManager {
    pub module_cache: ModuleCacheManager,

    pub lazy_compile_store: CacheStore,

    pub custom: CacheStore,

    pub lock: Mutex<bool>,
}

// impl CacheManager {
//     pub fn new() -> Self {
//         Self {

//         }
//     }
// }
