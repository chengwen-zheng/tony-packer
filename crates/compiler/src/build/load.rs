use std::sync::Arc;

use toy_farm_core::{
    error::Result, CompilationContext, CompilationError, PluginLoadHookParam, PluginLoadHookResult,
};

pub async fn load(
    load_param: &PluginLoadHookParam<'_>,
    context: &Arc<CompilationContext>,
) -> Result<PluginLoadHookResult> {
    let loaded = match context.plugin_driver.load(load_param, context).await {
        Ok(loaded) => match loaded {
            Some(loaded) => loaded,
            None => {
                return Err(CompilationError::LoadError {
                    resolved_path: load_param.module_id.to_string(),
                    source: None,
                });
            }
        },
        Err(e) => {
            return Err(CompilationError::LoadError {
                resolved_path: load_param.module_id.to_string(),
                source: Some(Box::new(e)),
            });
        }
    };

    Ok(loaded)
}
