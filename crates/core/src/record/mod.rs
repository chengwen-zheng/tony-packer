use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{ModuleId, ModuleType, PluginAnalyzeDepsHookResultEntry};

#[derive(Debug, Clone)]
pub enum Trigger {
    Compiler,
    Update,
}

#[allow(dead_code)]
/// All hook operation record are write down by [RecordManager]
pub struct RecordManager {
    resolve_id_map: Arc<RwLock<HashMap<String, Vec<ResolveRecord>>>>,
    transform_map: Arc<RwLock<HashMap<String, Vec<TransformRecord>>>>,
    process_map: Arc<RwLock<HashMap<String, Vec<ModuleRecord>>>>,
    analyze_deps_map: Arc<RwLock<HashMap<String, Vec<AnalyzeDepsRecord>>>>,
    resource_pot_map: Arc<RwLock<HashMap<String, Vec<ResourcePotRecord>>>>,
    pub plugin_stats: Arc<RwLock<HashMap<String, HashMap<String, PluginStats>>>>,
    trigger: Arc<RwLock<Trigger>>,
}

impl RecordManager {
    pub fn new() -> Self {
        Self {
            resolve_id_map: Arc::new(RwLock::new(HashMap::new())),
            transform_map: Arc::new(RwLock::new(HashMap::new())),
            process_map: Arc::new(RwLock::new(HashMap::new())),
            analyze_deps_map: Arc::new(RwLock::new(HashMap::new())),
            resource_pot_map: Arc::new(RwLock::new(HashMap::new())),
            plugin_stats: Arc::new(RwLock::new(HashMap::new())),
            trigger: Arc::new(RwLock::new(Trigger::Compiler)),
        }
    }

    pub async fn add_resolve_record(&self, source: String, mut record: ResolveRecord) {
        let mut resolve_id_map = self.resolve_id_map.write().await;
        self.update_plugin_stats(record.plugin.clone(), &record.hook.clone(), record.duration)
            .await;
        let trigger = self.trigger.read().await.to_owned();
        record.trigger = trigger;
        if let Some(records) = resolve_id_map.get_mut(&source) {
            records.push(record);
        } else {
            resolve_id_map.insert(source, vec![record]);
        }
    }

    pub async fn update_plugin_stats(&self, plugin_name: String, hook_name: &str, duration: i64) {
        let mut plugin_stats = self.plugin_stats.write().await;

        let plugin_entry = plugin_stats.entry(plugin_name.clone()).or_default();

        let stats = plugin_entry
            .entry(hook_name.to_string())
            .or_insert(PluginStats {
                total_duration: 0,
                call_count: 0,
            });

        stats.total_duration += duration;
        stats.call_count += 1;
    }

    pub async fn add_load_record(&self, id: String, mut record: TransformRecord) {
        let mut transform_map = self.transform_map.write().await;
        self.update_plugin_stats(record.plugin.clone(), &record.hook.clone(), record.duration)
            .await;
        let trigger = self.trigger.read().await.to_owned();
        record.trigger = trigger;
        if transform_map.get(&id).is_none() {
            transform_map.insert(id, vec![record]);
        }
    }

    pub async fn add_transform_record(&self, id: String, mut record: TransformRecord) {
        let mut transform_map = self.transform_map.write().await;
        self.update_plugin_stats(record.plugin.clone(), &record.hook.clone(), record.duration)
            .await;
        let trigger = self.trigger.read().await.to_owned();
        record.trigger = trigger;
        if let Some(records) = transform_map.get_mut(&id) {
            records.push(record);
        }
    }

    pub async fn add_parse_record(&self, id: String, mut record: ModuleRecord) {
        let mut process_map = self.process_map.write().await;
        self.update_plugin_stats(record.plugin.clone(), &record.hook.clone(), record.duration)
            .await;
        let trigger = self.trigger.read().await.to_owned();
        record.trigger = trigger;
        if process_map.get(&id).is_none() {
            process_map.insert(id, vec![record]);
        }
    }
}

impl Default for RecordManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ResolveRecord {
    pub plugin: String,
    pub hook: String,
    pub source: String,
    pub importer: Option<String>,
    pub kind: String,
    pub trigger: Trigger,
    pub start_time: i64,
    pub end_time: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct TransformRecord {
    pub plugin: String,
    pub hook: String,
    pub content: String,
    pub source_maps: Option<String>,
    pub module_type: ModuleType,
    pub trigger: Trigger,
    pub start_time: i64,
    pub end_time: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct ModuleRecord {
    pub plugin: String,
    pub hook: String,
    pub module_type: ModuleType,
    pub trigger: Trigger,
    pub start_time: i64,
    pub end_time: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct AnalyzeDepsRecord {
    pub plugin: String,
    pub hook: String,
    pub module_type: ModuleType,
    pub trigger: Trigger,
    pub deps: Vec<PluginAnalyzeDepsHookResultEntry>,
    pub start_time: i64,
    pub end_time: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct ResourcePotRecord {
    pub name: String,
    pub hook: String,
    pub modules: Vec<ModuleId>,
    pub resources: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStats {
    pub total_duration: i64,
    pub call_count: usize,
}
