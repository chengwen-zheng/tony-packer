use std::sync::Arc;

use toy_farm_core::error::Result;
use toy_farm_core::plugin::PluginResolveHookResult;
use toy_farm_core::{CompilationContext, PluginResolveHookParam};

pub fn resolve(
    resolve_param: &PluginResolveHookParam,
    context: &Arc<CompilationContext>,
) -> Result<PluginResolveHookResult> {
    let importer = resolve_param
        .importer
        .clone()
        .map(|p| p.to_string())
        .unwrap_or_else(|| context.config.root.clone());

    Ok(PluginResolveHookResult {
        resolved_path: importer,
        external: false,
        side_effects: true,
        query: vec![],
        meta: Default::default(),
    })
}
