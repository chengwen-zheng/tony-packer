use std::path::PathBuf;

use dashmap::DashMap;

#[allow(dead_code)]
#[derive(Default)]
pub struct CacheStore {
    cache_dir: PathBuf,
    /// name -> cache key manifest of this store.
    /// it will be stored in a separate file
    manifest: DashMap<String, String>,
}
