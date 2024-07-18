mod module_cached;
mod resolve;
use std::sync::Arc;

use crate::Compiler;

use resolve::resolve;
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};
use toy_farm_core::{
    error::Result, module::ModuleId, module_cache::CachedModule, plugin::PluginResolveHookResult,
    CompilationContext, CompilationError, Module, ModuleGraph, ModuleGraphEdgeDataItem,
    PluginAnalyzeDepsHookResultEntry, PluginResolveHookParam, ResolveKind,
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
    pub err_sender: Sender<CompilationError>,
}
pub(crate) struct HandleDependenciesParams {
    pub module: Module,
    pub resolve_param: PluginResolveHookParam,
    pub order: usize,
    pub deps: Vec<(PluginAnalyzeDepsHookResultEntry, Option<ModuleId>)>,
    // pub thread_pool: Arc<ThreadPool>,
    pub err_sender: Sender<CompilationError>,
    pub context: Arc<CompilationContext>,
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
                return Err(CompilationError::ResolveError("resolve error".to_string()));
            }
        };

        let module_id = get_module_id(&resolve_result);

        Ok(ResolveModuleIdResult {
            module_id,
            resolve_result,
        })
    }

    pub async fn build(&self) {
        let (err_sender, _err_receiver) = Self::create_thread_channel();

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
                err_sender: err_sender.clone(),
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
            err_sender,
        } = params;

        let resolve_module_result =
            match resolve_module(&resolve_param, cached_dependency, &context).await {
                Ok(result) => result,
                Err(e) => {
                    // log error
                    err_sender.send(e).await.unwrap();
                    return;
                }
            };

        match resolve_module_result {
            ResolveModuleResult::Success(resolved_module_info) => {
                let ResolvedModuleInfo {
                    mut module,
                    resolve_module_id_result,
                } = *resolved_module_info;
                if resolve_module_id_result.resolve_result.external {
                    // insert external module to the graph
                    let module_id = module.id.clone();
                    Self::add_module(module, &resolve_param.kind, &context).await;
                    Self::add_edge(&resolve_param, module_id, order, &context).await;
                    return;
                }

                // handle the resolved module
                match Self::build_module(
                    resolve_module_id_result.resolve_result,
                    &mut module,
                    &context,
                ) {
                    Err(e) => {
                        err_sender.send(e).await.unwrap();
                    }
                    Ok(deps) => {
                        let params = HandleDependenciesParams {
                            module,
                            resolve_param,
                            order,
                            deps,
                            err_sender,
                            context,
                        };
                        handle_dependencies(params).await;
                    }
                }
            }
            ResolveModuleResult::Built(module_id) => {
                // handle the built module
                Self::add_edge(&resolve_param, module_id, order, &context).await;
            }
            ResolveModuleResult::Cached(module_id) => {
                // handle the cached module
                let mut cached_module = context.cache_manager.module_cache.get_cache(&module_id);
                if let Err(e) = handle_cached_modules(&mut cached_module, &context).await {
                    err_sender.send(e).await.unwrap();
                };

                let params = HandleDependenciesParams {
                    module: cached_module.module,
                    resolve_param,
                    order,
                    deps: CachedModule::dep_sources(cached_module.dependencies),
                    // err_sender,
                    context,
                    err_sender,
                };

                handle_dependencies(params).await;
            }
        }
    }

    // async fn handle_dependencies(params: HandleDependenciesParams) {
    //     let HandleDependenciesParams {
    //         module,
    //         resolve_param,
    //         order,
    //         deps,
    //         err_sender,
    //         context,
    //     } = params;

    //     let module_id = module.id.clone();
    //     let immutable = module.immutable;
    //     // add module to the graph
    //     Self::add_module(module, &resolve_param.kind, &context).await;
    //     // add edge to the graph
    //     Self::add_edge(&resolve_param, module_id.clone(), order, &context).await;

    //     // resolving dependencies recursively in the thread pool

    //     // Resolve dependencies recursively in the thread pool
    //     let futures = deps
    //         .into_iter()
    //         .enumerate()
    //         .map(|(order, (dep, cached_dependency))| {
    //             let params = BuildModuleGraphParams {
    //                 resolve_param: PluginResolveHookParam {
    //                     source: dep.source,
    //                     importer: Some(module_id.clone()),
    //                     kind: dep.kind,
    //                 },
    //                 context: context.clone(),
    //                 err_sender: err_sender.clone(),
    //                 order,
    //                 cached_dependency: if immutable { cached_dependency } else { None },
    //             };
    //             tokio::spawn(async move { Self::build_module_graph(params).await })
    //         })
    //         .collect::<Vec<_>>();

    //     // Wait for all tasks to complete and handle any errors
    //     if let Err(e) = try_join_all(futures).await {
    //         let compilation_error = CompilationError::from(e);
    //         let _ = err_sender.send(compilation_error).await;
    //     }
    // }

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

    /// add a module to the module graph, if the module already exists, update it
    pub(crate) async fn add_module(
        module: Module,
        kind: &ResolveKind,
        context: &CompilationContext,
    ) {
        let mut module_graph = context.module_graph.write().await;

        // mark entry module
        if let ResolveKind::Entry(name) = kind {
            module_graph
                .entries
                .insert(module.id.clone(), name.to_string());
        }

        // check if the module already exists
        if module_graph.has_module(&module.id) {
            module_graph.replace_module(module);
        } else {
            module_graph.add_module(module);
        }
    }

    pub(crate) fn create_thread_channel() -> (Sender<CompilationError>, Receiver<CompilationError>)
    {
        let (err_sender, err_receiver) = channel::<CompilationError>(1024);

        (err_sender, err_receiver)
    }

    /// Resolving, loading, transforming and parsing a module, return the module and its dependencies if success
    pub(crate) fn build_module(
        _resolve_result: PluginResolveHookResult,
        _module: &mut Module,
        _context: &Arc<CompilationContext>,
    ) -> Result<Vec<(PluginAnalyzeDepsHookResultEntry, Option<ModuleId>)>> {
        todo!()
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

// This function prepares the parameters for each dependency
fn prepare_dependency_params(
    dep: PluginAnalyzeDepsHookResultEntry,
    cached_dependency: Option<ModuleId>,
    module_id: ModuleId,
    order: usize,
    context: Arc<CompilationContext>,
    err_sender: Sender<CompilationError>,
    immutable: bool,
) -> BuildModuleGraphParams {
    BuildModuleGraphParams {
        resolve_param: PluginResolveHookParam {
            source: dep.source,
            importer: Some(module_id),
            kind: dep.kind,
        },
        context,
        err_sender,
        order,
        cached_dependency: if immutable { cached_dependency } else { None },
    }
}

// This function spawns a task for a single dependency
fn spawn_dependency_task(
    params: BuildModuleGraphParams,
) -> JoinHandle<core::result::Result<(), CompilationError>> {
    tokio::spawn(async move {
        Compiler::build_module_graph(params).await;
        Ok(())
    })
}

async fn handle_dependencies(params: HandleDependenciesParams) {
    let HandleDependenciesParams {
        module,
        resolve_param,
        order,
        deps,
        err_sender,
        context,
    } = params;

    let module_id = module.id.clone();
    let immutable = module.immutable;

    // Add module to the graph
    Compiler::add_module(module, &resolve_param.kind, &context).await;
    // Add edge to the graph
    Compiler::add_edge(&resolve_param, module_id.clone(), order, &context).await;

    // Prepare parameters for each dependency
    let dependency_params: Vec<BuildModuleGraphParams> = deps
        .into_iter()
        .enumerate()
        .map(|(dep_order, (dep, cached_dependency))| {
            prepare_dependency_params(
                dep,
                cached_dependency,
                module_id.clone(),
                dep_order,
                Arc::clone(&context),
                err_sender.clone(),
                immutable,
            )
        })
        .collect();

    // Spawn tasks for each dependency
    let futures: Vec<JoinHandle<core::result::Result<(), CompilationError>>> = dependency_params
        .into_iter()
        .map(spawn_dependency_task)
        .collect();

    // Wait for all tasks to complete
    let results = futures::future::join_all(futures).await;

    // Handle errors
    for result in results {
        match result {
            Ok(Ok(())) => {} // Task completed successfully
            Ok(Err(compilation_error)) => {
                // Task returned a CompilationError
                if let Err(e) = err_sender.send(compilation_error).await {
                    eprintln!("Failed to send compilation error: {:?}", e);
                }
            }
            Err(join_error) => {
                // Task itself failed (e.g., panicked)
                let error = CompilationError::from(join_error);
                if let Err(e) = err_sender.send(error).await {
                    eprintln!("Failed to send join error: {:?}", e);
                }
            }
        }
    }
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
