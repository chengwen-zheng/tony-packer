use std::sync::Arc;

use toy_farm_core::{
    error::Result, CompilationContext, CompilationError, ModuleMetaData, PluginParseHookParam,
};

pub async fn parse(
    parse_param: &PluginParseHookParam,
    context: &Arc<CompilationContext>,
) -> Result<ModuleMetaData> {
    match context.plugin_driver.parse(parse_param, context).await {
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
