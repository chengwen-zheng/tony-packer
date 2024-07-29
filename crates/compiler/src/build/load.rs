use std::sync::Arc;

use toy_farm_core::{
    error::Result, CompilationContext, CompilationError, PluginLoadHookParam, PluginLoadHookResult,
};

pub async fn load(
    load_param: Arc<PluginLoadHookParam>,
    context: Arc<CompilationContext>,
) -> Result<PluginLoadHookResult> {
    let module_id = load_param.module_id.clone();
    let loaded = match context
        .clone()
        .plugin_driver
        .load(load_param, context)
        .await
    {
        Ok(loaded) => match loaded {
            Some(loaded) => loaded,
            None => {
                return Err(CompilationError::LoadError {
                    resolved_path: module_id,
                    source: None,
                });
            }
        },
        Err(e) => {
            return Err(CompilationError::LoadError {
                resolved_path: module_id,
                source: Some(Box::new(e)),
            });
        }
    };

    Ok(loaded)
}
