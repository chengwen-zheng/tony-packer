use std::sync::Arc;

use futures::Future;
use toy_farm_utils::stringify_query;

use crate::{
    error::Result,
    record::{ModuleRecord, ResolveRecord, TransformRecord, Trigger},
    CompilationContext, Config, ModuleMetaData, ModuleType, Plugin, PluginLoadHookParam,
    PluginLoadHookResult, PluginResolveHookParam, PluginResolveHookResult,
    PluginTransformHookParam, PluginTransformHookResult,
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{PluginParseHookParam, PluginProcessModuleHookParam};

macro_rules! hook_first {
  (
      $func_name:ident,
      $ret_ty:ty,
      $callback:expr,
      $($arg:ident: $ty:ty),*
  ) => {
      pub async fn $func_name(&self, $($arg: $ty),*) -> $ret_ty {
          for plugin in &self.plugins {
              let start_time = SystemTime::now()
                  .duration_since(UNIX_EPOCH)
                  .expect("Time went backwards")
                  .as_micros() as i64;

              let result = plugin.$func_name($($arg),*).await?;

              let end_time = SystemTime::now()
                  .duration_since(UNIX_EPOCH)
                  .expect("Failed to get end time")
                  .as_micros() as i64;

              if self.record {
                  let plugin_name = plugin.name().to_string();
                  $callback(
                      result.clone(),
                      plugin_name,
                      start_time,
                      end_time,
                      $($arg.clone()),*
                  ).await;
              }

              if result.is_some() {
                  return Ok(result);
              }
          }

          Ok(None)
      }
  };
}
macro_rules! hook_serial {
    ($func_name:ident, $param_ty:ty, $callback:expr) => {
        pub async fn $func_name(
            &self,
            param: $param_ty,
            context: &Arc<CompilationContext>,
        ) -> Result<()> {
            for plugin in &self.plugins {
                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_micros() as i64;

                plugin.$func_name(param, context).await?;

                let end_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("hook_serial get end_time failed")
                    .as_micros() as i64;

                if self.record {
                    let plugin_name = plugin.name().to_string();
                    let future = $callback(plugin_name, start_time, end_time, param, context);
                    future.await;
                }
            }

            Ok(())
        }
    };
}

pub struct PluginDriver {
    plugins: Vec<Box<dyn Plugin>>,
    record: bool,
}

impl PluginDriver {
    pub fn new(mut plugins: Vec<Box<dyn Plugin>>, record: bool) -> Self {
        plugins.sort_by_key(|b| std::cmp::Reverse(b.priority()));

        Self { plugins, record }
    }

    pub async fn config(&self, config: &mut Config) -> Result<()> {
        for plugin in &self.plugins {
            plugin.config(config).await?;
        }
        Ok(())
    }

    // MARK: RESOLVE
    hook_first!(
        resolve,
        Result<Option<PluginResolveHookResult>>,
        |result: Option<PluginResolveHookResult>,
        plugin_name: String,
        start_time: i64,
        end_time: i64,
        param: PluginResolveHookParam,
        context: Arc<CompilationContext>| async move {
            if let Some(resolve_result) = result {
                context.record_manager.add_resolve_record(
                    resolve_result.resolved_path.clone() + stringify_query(&resolve_result.query).as_str(),
                    ResolveRecord {
                        start_time,
                        end_time,
                        duration: end_time - start_time,
                        plugin: plugin_name,
                        hook: "resolve".to_string(),
                        source: param.source.clone(),
                        importer: param
                            .importer
                            .clone()
                            .map(|module_id| module_id.relative_path().to_string()),
                        kind: String::from(param.kind.clone()),
                        trigger: Trigger::Compiler,
                    },
                ).await;
            }
        },
        param: &PluginResolveHookParam,
        context: &Arc<CompilationContext>
    );

    // dont't use macro here and support async closure
    // pub async fn resolve<F>(
    //     &self,
    //     param: &PluginResolveHookParam,
    //     context: &Arc<CompilationContext>,
    // ) -> Result<Option<PluginResolveHookResult>> {
    //     let callback = |result: Option<PluginResolveHookResult>,
    //                     plugin_name: String,
    //                     start_time: i64,
    //                     end_time: i64,
    //                     param: PluginResolveHookParam,
    //                     context: Arc<CompilationContext>| async move {
    //         if let Some(resolve_result) = result {
    //             context
    //                 .record_manager
    //                 .add_resolve_record(
    //                     resolve_result.resolved_path.clone()
    //                         + stringify_query(&resolve_result.query).as_str(),
    //                     ResolveRecord {
    //                         start_time,
    //                         end_time,
    //                         duration: end_time - start_time,
    //                         plugin: plugin_name,
    //                         hook: "resolve".to_string(),
    //                         source: param.source.clone(),
    //                         importer: param
    //                             .importer
    //                             .clone()
    //                             .map(|module_id| module_id.relative_path().to_string()),
    //                         kind: String::from(param.kind.clone()),
    //                         trigger: Trigger::Compiler,
    //                     },
    //                 )
    //                 .await;
    //         }
    //     };

    //     for plugin in &self.plugins {
    //         let start_time = SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .expect("Time went backwards")
    //             .as_micros() as i64;

    //         let result = plugin.resolve(param, context).await?;

    //         let end_time = SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .expect("Failed to get end time")
    //             .as_micros() as i64;

    //         if self.record {
    //             let plugin_name = plugin.name().to_string();
    //             callback(
    //                 result.clone(),
    //                 plugin_name,
    //                 start_time,
    //                 end_time,
    //                 param.clone(),
    //                 context.clone(),
    //             )
    //             .await;
    //         }

    //         if result.is_some() {
    //             return Ok(result);
    //         }
    //     }

    //     Ok(None)
    // }

    // MARK: LOAD

    hook_first!(
        load,
        Result<Option<PluginLoadHookResult>>,
        |result: Option<PluginLoadHookResult>,
         plugin_name: String,
         start_time: i64,
         end_time: i64,
         param: PluginLoadHookParam<'_>,
         context: Arc<CompilationContext>|
         {
            let resolved_path = param.resolved_path.to_string();
            let query = param.query.clone();
            async move {
                if let Some(load_result) = result {
                    let full_path = format!("{}{}", resolved_path, stringify_query(&query));

                    context
                        .record_manager
                        .add_load_record(
                            full_path,
                            TransformRecord {
                                plugin: plugin_name,
                                hook: "load".to_string(),
                                content: load_result.content.clone(),
                                source_maps: None,
                                module_type: load_result.module_type.clone(),
                                trigger: Trigger::Compiler,
                                start_time,
                                end_time,
                                duration: end_time - start_time,
                            },
                        )
                        .await;
                }
            }
         }
         ,
        param: &PluginLoadHookParam<'_>,
        context: &Arc<CompilationContext>
    );
    // Don't use macro here and support async closure
    // pub async fn load(
    //     &self,
    //     param: &PluginLoadHookParam<'_>,
    //     context: &Arc<CompilationContext>,
    // ) -> Result<Option<PluginLoadHookResult>> {
    //     for plugin in &self.plugins {
    //         let start_time = SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .expect("Time went backwards")
    //             .as_micros() as i64;

    //         let result = plugin.load(param, context).await?;

    //         let end_time = SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .expect("Failed to get end time")
    //             .as_micros() as i64;

    //         if self.record {
    //             let plugin_name = plugin.name().to_string();
    //             match result {
    //                 Some(ref load_result) => {
    //                     context
    //                         .record_manager
    //                         .add_load_record(
    //                             param.resolved_path.to_string() + stringify_query(&param.query).as_str(),
    //                             TransformRecord {
    //                                 plugin: plugin_name,
    //                                 hook: "load".to_string(),
    //                                 content: load_result.content.clone(),
    //                                 source_maps: None,
    //                                 module_type: load_result.module_type.clone(),
    //                                 trigger: Trigger::Compiler,
    //                                 start_time,
    //                                 end_time,
    //                                 duration: end_time - start_time,
    //                             },
    //                         )
    //                         .await;
    //                 }
    //                 None => {}
    //             }
    //         }

    //         if result.is_some() {
    //             return Ok(result);
    //         }
    //     }

    //     Ok(None)
    // }

    // MARK: TRANSFORM
    pub async fn transform(
        &self,
        mut param: PluginTransformHookParam<'_>,
        context: &Arc<CompilationContext>,
    ) -> Result<PluginDriverTransformHookResult> {
        let mut result = PluginDriverTransformHookResult {
            content: String::new(),
            source_map_chain: param.source_map_chain.clone(),
            module_type: None,
        };

        let transform_results = self.apply_transforms(&mut param, context).await?;

        for (plugin_name, plugin_result, duration) in transform_results {
            self.update_result(&mut result, &mut param, plugin_result);
            self.record_transform(plugin_name, &param, &result, duration, context)
                .await;
        }

        result.content = param.content;
        result.module_type = Some(param.module_type);

        Ok(result)
    }

    async fn apply_transforms(
        &self,
        param: &mut PluginTransformHookParam<'_>,
        context: &Arc<CompilationContext>,
    ) -> Result<Vec<(String, Option<PluginTransformHookResult>, Option<i64>)>> {
        let mut results = Vec::new();

        for plugin in &self.plugins {
            let transform_future = plugin.transform(param, context);
            let (start_time, plugin_result) = self.measure_time(transform_future).await;
            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_micros() as i64;
            let duration = start_time.map(|start| end_time - start);

            results.push((plugin.name().to_string(), plugin_result?, duration));
        }

        Ok(results)
    }

    async fn measure_time<F>(&self, future: F) -> (Option<i64>, F::Output)
    where
        F: Future,
    {
        let start_time = self.record.then(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_micros() as i64
        });

        let result = future.await;

        (start_time, result)
    }

    fn update_result(
        &self,
        result: &mut PluginDriverTransformHookResult,
        param: &mut PluginTransformHookParam<'_>,
        plugin_result: Option<PluginTransformHookResult>,
    ) {
        let Some(plugin_result) = plugin_result else {
            return;
        };

        param.content = plugin_result.content;
        param.module_type = plugin_result
            .module_type
            .unwrap_or(param.module_type.clone());

        if plugin_result.ignore_previous_source_map {
            result.source_map_chain.clear();
        }

        if let Some(source_map) = plugin_result.source_map {
            let sourcemap = Arc::new(source_map);
            result.source_map_chain.push(sourcemap.clone());
            param.source_map_chain.push(sourcemap);
        }
    }

    async fn record_transform(
        &self,
        plugin_name: String,
        param: &PluginTransformHookParam<'_>,
        result: &PluginDriverTransformHookResult,
        duration: Option<i64>,
        context: &Arc<CompilationContext>,
    ) {
        if !self.record {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        let (start_time, end_time) = duration.map_or((now, now), |d| (now - d, now));

        context
            .record_manager
            .add_transform_record(
                param.resolved_path.to_string() + stringify_query(&param.query).as_str(),
                TransformRecord {
                    plugin: plugin_name,
                    hook: "transform".to_string(),
                    content: param.content.clone(),
                    source_maps: result
                        .source_map_chain
                        .last()
                        .map(|arc| arc.as_ref().clone()),
                    module_type: param.module_type.clone(),
                    trigger: Trigger::Compiler,
                    start_time,
                    end_time,
                    duration: duration.unwrap_or(0),
                },
            )
            .await;
    }

    // MARK: PARSE
    hook_first!(
        parse,
        Result<Option<ModuleMetaData>>,
        |_result: Option<ModuleMetaData>,
         plugin_name: String,
         start_time: i64,
         end_time: i64,
         param: PluginParseHookParam,
         context: Arc<CompilationContext>|
         {
            let resolved_path = param.resolved_path.to_string();
            let query = param.query.clone();
            async move {
                let full_path = format!("{}{}", resolved_path, stringify_query(&query));

                context
                    .record_manager
                    .add_parse_record(
                        full_path,
                        ModuleRecord {
                            plugin: plugin_name,
                            hook: "parse".to_string(),
                            module_type: param.module_type.clone(),
                            trigger: Trigger::Compiler,
                            start_time,
                            end_time,
                            duration: end_time - start_time,
                        },
                    )
                    .await;
            }
         }
         ,
        param: &PluginParseHookParam,
        context: &Arc<CompilationContext>
    );

    // MARK: PROCESS_MODULE
    hook_serial!(
        process_module,
        &mut PluginProcessModuleHookParam<'_>,
        |plugin_name: String,
         start_time: i64,
         end_time: i64,
         param: &PluginProcessModuleHookParam,
         context: &Arc<CompilationContext>| {
            let resolved_path = param.module_id.resolved_path(&context.config.root);
            let query = param.module_id.query_string();
            let module_type = param.module_type.clone();
            let full_path = format!("{}{}", resolved_path, query);
            let context = context.clone();
            async move {
                context
                    .add_process_record(
                        full_path,
                        ModuleRecord {
                            plugin: plugin_name,
                            hook: "process".to_string(),
                            module_type,
                            trigger: Trigger::Compiler,
                            start_time,
                            end_time,
                            duration: end_time - start_time,
                        },
                    )
                    .await;
            }
        }
    );
}

#[derive(Debug, Clone)]
pub struct PluginDriverTransformHookResult {
    pub content: String,
    pub source_map_chain: Vec<Arc<String>>,
    pub module_type: Option<ModuleType>,
}
