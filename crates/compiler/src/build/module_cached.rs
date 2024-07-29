use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};

use toy_farm_core::{
    error::Result,
    module_cache::{CachedModule, CachedWatchDependency},
    CompilationContext, ModuleId, ModuleMetaData,
};
use toy_farm_utils::hash;

pub fn get_timestamp_of_module(module_id: &ModuleId, root: &str) -> u128 {
    let resolved_path = module_id.resolved_path(root);

    if !PathBuf::from(&resolved_path).exists() {
        // return unix epoch if the module is not found
        return SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
    }

    let file_meta = std::fs::metadata(resolved_path).unwrap_or_else(|_| {
        panic!(
            "Failed to get metadata of module {:?}",
            module_id.resolved_path(root)
        )
    });
    let system_time = file_meta.modified();

    if let Ok(system_time) = system_time {
        if let Ok(dur) = system_time.duration_since(SystemTime::UNIX_EPOCH) {
            return dur.as_nanos();
        }
    }

    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

pub fn get_content_hash_of_module(content: &str) -> String {
    let content = if content.is_empty() {
        "empty".to_string()
    } else {
        content.to_string()
    };

    let module_content_hash = hash::sha256(content.as_bytes(), 32);
    module_content_hash
}

pub async fn try_get_module_cache_by_hash(
    module_id: &ModuleId,
    content_hash: &str,
    context: &Arc<CompilationContext>,
) -> toy_farm_core::error::Result<Option<CachedModule>> {
    let mut should_invalidate_cache = false;

    if context.config.persistent_cache.hash_enabled()
        && context.cache_manager.module_cache.has_cache(module_id)
    {
        let cached_module = context.cache_manager.module_cache.get_cache_ref(module_id);

        if cached_module.value().module.content_hash == content_hash {
            drop(cached_module);
            let mut cached_module = context.cache_manager.module_cache.get_cache(module_id);

            handle_cached_modules(&mut cached_module, context).await?;

            if cached_module.module.immutable
                || !is_watch_dependencies_content_hash_changed(&cached_module, context).await
            {
                // TODO: handle persistent cached module
                let should_invalidate_cached_module = false;

                if !should_invalidate_cached_module {
                    return Ok(Some(cached_module));
                } else {
                    should_invalidate_cache = true;
                }
            }
        } else {
            should_invalidate_cache = true;
        }
    }

    if should_invalidate_cache {
        context
            .cache_manager
            .module_cache
            .invalidate_cache(module_id);
    }

    Ok(None)
}

pub async fn try_get_module_cache_by_timestamp(
    module_id: &ModuleId,
    timestamp: u128,
    context: Arc<CompilationContext>,
) -> Result<Option<CachedModule>> {
    let mut should_invalidate_cache = false;

    if context.config.persistent_cache.timestamp_enabled()
        && context.cache_manager.module_cache.has_cache(module_id)
    {
        let cached_module = context.cache_manager.module_cache.get_cache_ref(module_id);

        if cached_module.value().module.last_update_timestamp == timestamp {
            drop(cached_module);
            let mut cached_module = context.cache_manager.module_cache.get_cache(module_id);
            handle_cached_modules(&mut cached_module, &context).await?;

            if cached_module.module.immutable
                || !is_watch_dependencies_timestamp_changed(&cached_module, &context).await
            {
                // TODO: handle persistent cached module
                let should_invalidate_cached_module = false;

                if !should_invalidate_cached_module {
                    return Ok(Some(cached_module));
                } else {
                    should_invalidate_cache = true;
                }
            }
        } else if !context.config.persistent_cache.hash_enabled() {
            should_invalidate_cache = true;
        }
    }

    if should_invalidate_cache {
        context
            .cache_manager
            .module_cache
            .invalidate_cache(module_id);
    }

    Ok(None)
}
async fn handle_relation_roots(
    cached_module_id: &ModuleId,
    watch_dependencies: &[CachedWatchDependency],
    context: &Arc<CompilationContext>,
) -> Result<()> {
    if !watch_dependencies.is_empty() {
        let mut watch_graph = context.watch_graph.write().await;
        watch_graph.add_node(cached_module_id.clone());

        for cached_dep in watch_dependencies {
            let dep = &cached_dep.dependency;
            watch_graph.add_node(dep.clone());
            watch_graph.add_edge(cached_module_id, dep)?;
        }
    }

    Ok(())
}

pub async fn handle_cached_modules(
    cached_module: &mut CachedModule,
    context: &Arc<CompilationContext>,
) -> Result<()> {
    // using swc resolver
    match &mut cached_module.module.meta.as_mut() {
        ModuleMetaData::Script(script) => {
            // 重置标记以防止标记被重用，稍后会重新解析
            script.top_level_mark = 0;
            script.unresolved_mark = 0;
        }
        ModuleMetaData::Css { .. } => { /* 不做任何事 */ }
        ModuleMetaData::Html { .. } => { /* 不做任何事 */ }
        ModuleMetaData::Custom { .. } => { /* TODO: 为自定义模块添加一个钩子 */ }
    };

    handle_relation_roots(
        &cached_module.module.id,
        &cached_module.watch_dependencies,
        context,
    )
    .await?;

    Ok(())
}

async fn is_watch_dependencies_timestamp_changed(
    cached_module: &CachedModule,
    context: &Arc<CompilationContext>,
) -> bool {
    let watch_graph = context.watch_graph.read().await;
    let relation_dependencies = watch_graph.relation_dependencies(&cached_module.module.id);

    if relation_dependencies.is_empty() {
        return false;
    }

    let cached_dep_timestamp_map = cached_module
        .watch_dependencies
        .iter()
        .map(|dep| (dep.dependency.clone(), dep.timestamp))
        .collect::<HashMap<_, _>>();

    for dep in &relation_dependencies {
        let resolved_path = PathBuf::from(dep.resolved_path(&context.config.root));
        let cached_timestamp = cached_dep_timestamp_map.get(dep);

        if !resolved_path.exists()
            || cached_timestamp.is_none()
            || get_timestamp_of_module(dep, &context.config.root) != *cached_timestamp.unwrap()
        {
            return true;
        }
    }

    false
}

async fn is_watch_dependencies_content_hash_changed(
    cached_module: &CachedModule,
    context: &Arc<CompilationContext>,
) -> bool {
    let watch_graph = context.watch_graph.read().await;
    let relation_dependencies = watch_graph.relation_dependencies(&cached_module.module.id);

    if relation_dependencies.is_empty() {
        return false;
    }

    let cached_dep_hash_map = cached_module
        .watch_dependencies
        .iter()
        .map(|dep| (dep.dependency.clone(), dep.hash.clone()))
        .collect::<HashMap<_, _>>();

    for dep in relation_dependencies {
        let resolved_path = PathBuf::from(dep.resolved_path(&context.config.root));
        let cached_hash = cached_dep_hash_map.get(dep);

        if !resolved_path.exists() || cached_hash.is_none() {
            return true;
        }

        let content = std::fs::read_to_string(resolved_path).unwrap();
        let hash = get_content_hash_of_module(&content);

        if hash != *cached_hash.unwrap() {
            return true;
        }
    }

    false
}
