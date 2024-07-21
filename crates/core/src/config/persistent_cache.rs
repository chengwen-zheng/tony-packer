use std::collections::HashMap;

use relative_path::RelativePath;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PersistentCacheConfig {
    Bool(bool),
    Obj(PersistentCacheConfigObj),
}

impl PersistentCacheConfig {
    pub fn timestamp_enabled(&self) -> bool {
        match self {
            PersistentCacheConfig::Bool(b) => *b,
            PersistentCacheConfig::Obj(obj) => obj.module_cache_key_strategy.timestamp,
        }
    }

    pub fn hash_enabled(&self) -> bool {
        match self {
            PersistentCacheConfig::Bool(b) => *b,
            PersistentCacheConfig::Obj(obj) => obj.module_cache_key_strategy.hash,
        }
    }

    pub fn get_default_config(root: &str) -> Self {
        let cache_dir = RelativePath::new("node_modules/.farm/cache")
            .to_logical_path(root)
            .to_string_lossy()
            .to_string();

        PersistentCacheConfig::Obj(PersistentCacheConfigObj {
            namespace: "farm-cache".to_string(),
            cache_dir,
            module_cache_key_strategy: PersistentModuleCacheKeyStrategy {
                timestamp: true,
                hash: true,
            },
            // build dependencies are set by node side
            build_dependencies: vec![],
            envs: HashMap::new(),
        })
    }
    pub fn enabled(&self) -> bool {
        match self {
            Self::Bool(enabled) => *enabled,
            Self::Obj(_) => true,
        }
    }

    pub fn as_obj(&self, root: &str) -> PersistentCacheConfigObj {
        match self {
            Self::Obj(obj) => obj.clone(),
            Self::Bool(_) => PersistentCacheConfigObj {
                cache_dir: format!("{}/.cache", root),
                namespace: "default".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistentCacheConfigObj {
    pub namespace: String,
    pub cache_dir: String,
    pub module_cache_key_strategy: PersistentModuleCacheKeyStrategy,
    /// If the build dependencies changed, the cache need to be invalidated. The value must be absolute path.
    /// It's absolute paths of farm.config by default. Farm will use their timestamp and hash to invalidate cache.
    /// Note that farm will resolve the config file dependencies from node side
    pub build_dependencies: Vec<String>,
    pub envs: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistentModuleCacheKeyStrategy {
    pub timestamp: bool,
    pub hash: bool,
}

impl Default for PersistentModuleCacheKeyStrategy {
    fn default() -> Self {
        Self {
            timestamp: true,
            hash: true,
        }
    }
}
