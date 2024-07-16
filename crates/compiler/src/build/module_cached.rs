use std::sync::Arc;

use toy_farm_core::{
    error::Result,
    module_cache::{CachedModule, CachedWatchDependency},
    CompilationContext, ModuleId, ModuleMetaData,
};
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
