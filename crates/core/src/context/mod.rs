use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    persistent_cache::PersistentCacheConfig,
    plugin_driver::PluginDriver,
    record::{ModuleRecord, RecordManager},
    watch_graph::WatchGraph,
    CacheManager, Config, ModuleGraph, Plugin,
};

pub struct CompilationContext {
    pub module_graph: Box<RwLock<ModuleGraph>>,
    pub config: Box<Config>,
    pub cache_manager: Box<CacheManager>,
    pub watch_graph: Box<RwLock<WatchGraph>>,
    pub record_manager: Box<RecordManager>,
    pub plugin_driver: Box<PluginDriver>,
}

pub(crate) const EMPTY_STR: &str = "";

impl CompilationContext {
    pub fn new(mut config: Config, plugins: Vec<Arc<dyn Plugin>>) -> CompilationContext {
        let (cache_dir, namespace) =
            CompilationContext::normalize_persistent_cache_config(&mut config);
        CompilationContext {
            module_graph: Box::new(RwLock::new(ModuleGraph::new())),
            cache_manager: Box::new(CacheManager::new(
                &cache_dir,
                &namespace,
                config.mode.clone(),
            )),
            plugin_driver: Box::new(PluginDriver::new(plugins, config.record)),
            config: Box::new(config),
            watch_graph: Box::new(RwLock::new(WatchGraph::new())),
            record_manager: Box::new(RecordManager::new()),
        }
    }

    pub fn normalize_persistent_cache_config(config: &mut Config) -> (String, String) {
        if config.persistent_cache.enabled() {
            let cache_config_obj = config.persistent_cache.as_obj(&config.root);
            let (cache_dir, namespace) = (
                cache_config_obj.cache_dir.clone(),
                cache_config_obj.namespace.clone(),
            );
            config.persistent_cache = Box::new(PersistentCacheConfig::Obj(cache_config_obj));

            (cache_dir, namespace)
        } else {
            (EMPTY_STR.to_string(), EMPTY_STR.to_string())
        }
    }

    pub async fn add_process_record(&self, key: String, record: ModuleRecord) {
        self.record_manager.add_process_record(key, record).await;
    }
}
