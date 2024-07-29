use std::sync::Arc;

use toy_farm_core::{
    error::Result, CompilationContext, CompilationError, ModuleMetaData, PluginParseHookParam,
};

pub async fn parse(
    parse_param: Arc<PluginParseHookParam>,
    context: &Arc<CompilationContext>,
) -> Result<ModuleMetaData> {
    match context
        .clone()
        .plugin_driver
        .parse(parse_param.clone(), context.clone())
        .await
    {
        Ok(meta) => match meta {
            Some(meta) => Ok(meta),
            None => Err(CompilationError::ParseError {
                resolved_path: parse_param.module_id.to_string(),
                msg: format!(
                    "No plugins handle this kind of module: {:?}",
                    parse_param.module_type
                ),
            }),
        },
        Err(e) => Err(CompilationError::ParseError {
            resolved_path: parse_param.module_id.to_string(),
            msg: e.to_string(),
        }),
    }
}
