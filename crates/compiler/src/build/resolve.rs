use std::sync::Arc;

use toy_farm_core::error::Result;
use toy_farm_core::plugin::PluginResolveHookResult;
use toy_farm_core::{CompilationContext, CompilationError, PluginResolveHookParam};

pub async fn resolve(
    resolve_param: &PluginResolveHookParam,
    context: &Arc<CompilationContext>,
) -> Result<PluginResolveHookResult> {
    let importer = resolve_param
        .importer
        .clone()
        .map(|p| p.to_string())
        .unwrap_or_else(|| context.config.root.clone());

    let resolved = match context.plugin_driver.resolve(resolve_param, context).await {
        Ok(resolved) => match resolved {
            Some(res) => res,
            None => {
                return Err(CompilationError::ResolveError {
                    importer,
                    src: resolve_param.source.clone(),
                    source: None,
                });
            }
        },
        Err(e) => {
            return Err(CompilationError::ResolveError {
                importer,
                src: resolve_param.source.clone(),
                source: Some(Box::new(e)),
            });
        }
    };

    Ok(resolved)
}
