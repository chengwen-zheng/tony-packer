use std::sync::Arc;

use toy_farm_core::{
    error::Result, plugin_driver::PluginDriverTransformHookResult, CompilationContext,
    CompilationError, PluginTransformHookParam,
};

pub async fn transform(
    transform_param: PluginTransformHookParam,
    context: Arc<CompilationContext>,
) -> Result<PluginDriverTransformHookResult> {
    let module_id = transform_param.module_id.to_string();
    let transformed = context
        .plugin_driver
        .transform(transform_param, context.clone())
        .await
        .map_err(|e| CompilationError::TransformError {
            resolved_path: module_id,
            msg: e.to_string(),
        })?;

    Ok(transformed)
}
