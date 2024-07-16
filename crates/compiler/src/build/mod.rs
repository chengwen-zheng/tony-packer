mod module_cached;
mod resolve;
use std::sync::Arc;

use crate::Compiler;

use resolve::resolve;
use toy_farm_core::{
    error::Result, module::ModuleId, plugin::PluginResolveHookResult, CompilationContext, Module,
    ModuleGraph, ModuleGraphEdgeDataItem, PluginResolveHookParam, ResolveKind,
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
    // The module is already built
    Built(ModuleId),
    Cached(ModuleId),
    Success(Box<ResolvedModuleInfo>),
}

pub(crate) struct BuildModuleGraphParams {
    pub resolve_param: PluginResolveHookParam,
    pub context: Arc<CompilationContext>,
    pub cached_dependency: Option<ModuleId>,
    pub order: usize,
}

use self::module_cached::handle_cached_modules;

impl Compiler {
    fn resolve_module_id(
        _resolve_param: &PluginResolveHookParam,
        _context: &Arc<CompilationContext>,
    ) -> Result<ResolveModuleIdResult> {
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
                cached_dependency: None,
                order,
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
            cached_dependency,
            order,
        } = params;

        let resolve_module_result =
            match resolve_module(&resolve_param, cached_dependency, &context).await {
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
            ResolveModuleResult::Built(module_id) => {
                // handle the built module
                Self::add_edge(&resolve_param, module_id, order, &context).await;
            }
            ResolveModuleResult::Cached(module_id) => {
                // handle the cached module
                let mut cached_module = context.cache_manager.module_cache.get_cache(&module_id);
                handle_cached_modules(&mut cached_module, &context)
                    .await
                    .unwrap();
            }
        }
    }

    async fn handle_dependencies(
        _resolve_module_id_result: ResolveModuleIdResult,
        _context: &Arc<CompilationContext>,
    ) {
        todo!();
    }

    async fn add_edge(
        resolve_param: &PluginResolveHookParam,
        module_id: ModuleId,
        order: usize,
        context: &CompilationContext,
    ) {
        let mut module_graph = context.module_graph.write().await;
        if let Some(importer_id) = &resolve_param.importer {
            module_graph.add_edge_item(
              importer_id,
              &module_id,
              ModuleGraphEdgeDataItem {
                source: resolve_param.source.clone(),
                kind: resolve_param.kind.clone(),
                order,
              },
            ).expect("failed to add edge to the module graph, the endpoint modules of the edge should be in the graph")
        }
    }
}

fn handle_cached_dependency(
    cached_dependency: &ModuleId,
    module_graph: &mut ModuleGraph,
    context: &Arc<CompilationContext>,
) -> Result<Option<ResolveModuleResult>> {
    let module_cache_manager = &context.cache_manager.module_cache;

    if module_cache_manager.has_cache(cached_dependency) {
        // todo: to finish plugin driver and handle persistent cache
        let _cached_module = module_cache_manager.get_cache_ref(cached_dependency);
        let should_invalidate_cached_module = true;

        if should_invalidate_cached_module {
            module_cache_manager.invalidate_cache(cached_dependency);
        } else {
            Compiler::insert_dummy_module(cached_dependency, module_graph);
            return Ok(Some(ResolveModuleResult::Cached(cached_dependency.clone())));
        }
    }

    Ok(None)
}

async fn resolve_module(
    resolve_param: &PluginResolveHookParam,
    cached_dependency: Option<ModuleId>,
    context: &Arc<CompilationContext>,
) -> Result<ResolveModuleResult> {
    let resolve_module_id_result = cached_dependency.clone().map_or_else(
        || Compiler::resolve_module_id(resolve_param, context).map(Some),
        |_| Ok(None),
    )?;

    let module_id = cached_dependency
        .clone()
        .unwrap_or_else(|| resolve_module_id_result.as_ref().unwrap().module_id.clone());

    let mut module_graph: tokio::sync::RwLockWriteGuard<ModuleGraph> =
        context.module_graph.write().await;

    if module_graph.has_module(&module_id) {
        return Ok(ResolveModuleResult::Built(module_id));
    }

    if let Some(cached_dependency) = cached_dependency {
        if let Some(result) =
            handle_cached_dependency(&cached_dependency, &mut module_graph, context)?
        {
            return Ok(result);
        }
    }

    let resolve_module_id_result = resolve_module_id_result
        .unwrap_or_else(|| Compiler::resolve_module_id(resolve_param, context).unwrap());

    Compiler::insert_dummy_module(&resolve_module_id_result.module_id, &mut module_graph);

    // todo: handle immutable modules
    // let module_id_str = resolve_module_id_result.module_id.to_string();
    // let immutable = !module_id_str.ends_with(DYNAMIC_VIRTUAL_SUFFIX) &&
    // context.config.partial_bundling.immutable_modules.iter().any(|im| im.is_match(&module_id_str)),

    let module = Compiler::create_module(
        resolve_module_id_result.module_id.clone(),
        resolve_module_id_result.resolve_result.external,
        false,
    );

    Ok(ResolveModuleResult::Success(Box::new(ResolvedModuleInfo {
        module,
        resolve_module_id_result,
    })))
}
