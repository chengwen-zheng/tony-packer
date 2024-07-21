use std::sync::Arc;

use toy_farm_utils::stringify_query;

use crate::{
    error::Result,
    record::{ResolveRecord, Trigger},
    CompilationContext, Config, Plugin, PluginResolveHookParam, PluginResolveHookResult,
};
use std::time::{SystemTime, UNIX_EPOCH};

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
}
