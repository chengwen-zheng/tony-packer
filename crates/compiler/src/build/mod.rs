mod resolve;
use std::sync::Arc;

use crate::Compiler;

use resolve::resolve;
use toy_farm_core::{
    error::Result, module::ModuleId, plugin::PluginResolveHookResult, CompilationContext, Module,
    ModuleGraph, PluginResolveHookParam, ResolveKind,
};
use toy_farm_utils::stringify_query;
#[derive(Debug)]
pub(crate) struct ResolveModuleIdResult {
    pub module_id: ModuleId,
    pub resolve_result: PluginResolveHookResult,
}
pub(crate) struct ResolvedModuleInfo {
    pub module: Module,
    pub resolve_module_id_result: ResolveModuleIdResult,
}

enum ResolveModuleResult {
    /// The module is already built
    // Built(ModuleId),
    // Cached(ModuleId),
    Success(Box<ResolvedModuleInfo>),
}

struct BuildModuleGraphParams {
    resolve_param: PluginResolveHookParam,
    context: Arc<CompilationContext>,
}

impl Compiler {
    fn resolve_module_id(_resolve_param: &PluginResolveHookParam) -> Result<ResolveModuleIdResult> {
        let get_module_id = |resolve_result: &PluginResolveHookResult| {
            // make query part of module id
            ModuleId::new(
                &resolve_result.resolved_path,
                &stringify_query(&resolve_result.query),
            )
        };

        let resolve_result = match resolve() {
            Ok(result) => result,
            Err(_) => {
                // log error
                todo!();
            }
        };

        let module_id = get_module_id(&resolve_result);

        Ok(ResolveModuleIdResult {
            module_id,
            resolve_result,
        })
    }

    pub async fn build(&self) {
        for (order, (name, source)) in self.context.config.input.iter().enumerate() {
            println!("Index: {}, Name: {}, Source: {}", order, name, source);

            let resolve_param = PluginResolveHookParam {
                kind: ResolveKind::Entry(name.clone()),
                source: source.clone(),
                importer: None,
            };

            let build_module_graph_params = BuildModuleGraphParams {
                resolve_param,
                context: self.context.clone(),
            };

            Compiler::build_module_graph(build_module_graph_params).await;
        }
    }

    pub(crate) fn create_module(module_id: ModuleId, external: bool, immutable: bool) -> Module {
        let mut module = Module::new(module_id);

        // if the module is external, return a external module
        if external {
            module.external = true;
        }

        if immutable {
            module.immutable = true;
        }

        module
    }

    pub(crate) fn insert_dummy_module(module_id: &ModuleId, module_graph: &mut ModuleGraph) {
        // insert a dummy module to the graph to prevent the module from being handled twice
        module_graph.add_module(Compiler::create_module(module_id.clone(), false, false));
    }

    async fn build_module_graph(params: BuildModuleGraphParams) {
        // build module graph
        let BuildModuleGraphParams {
            resolve_param,
            context,
        } = params;

        let resolve_module_result = match resolve_module(&resolve_param, &context).await {
            Ok(result) => result,
            Err(_) => {
                // log error
                todo!();
            }
        };

        match resolve_module_result {
            ResolveModuleResult::Success(resolved_module_info) => {
                let ResolvedModuleInfo {
                    module,
                    resolve_module_id_result,
                } = *resolved_module_info;

                let mut module_graph = context.module_graph.write().await;
                module_graph.add_module(module);

                // handle the resolved module

                Compiler::handle_dependencies(resolve_module_id_result, &context).await;
            }
        }
    }

    async fn handle_dependencies(
        _resolve_module_id_result: ResolveModuleIdResult,
        _context: &Arc<CompilationContext>,
    ) {
        todo!();
    }
}

async fn resolve_module(
    resolve_param: &PluginResolveHookParam,
    context: &Arc<CompilationContext>,
) -> Result<ResolveModuleResult> {
    let resolve_module_id_result = match Compiler::resolve_module_id(resolve_param) {
        Ok(result) => result,
        Err(_) => {
            // log error
            todo!();
        }
    };

    let mut module_graph = context.module_graph.write().await;
    Compiler::insert_dummy_module(&resolve_module_id_result.module_id, &mut module_graph);

    let res = ResolveModuleResult::Success(Box::new(ResolvedModuleInfo {
        module: Compiler::create_module(
            resolve_module_id_result.module_id.clone(),
            resolve_module_id_result.resolve_result.external,
            // treat all lazy virtual modules as mutable
            false,
        ),
        resolve_module_id_result,
    }));

    Ok(res)
}
