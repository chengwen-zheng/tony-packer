use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    vec,
};

use async_trait::async_trait;
use tokio::sync::RwLock;
use toy_farm_core::{
    error::Result, external::ExternalConfig, CompilationContext, Config, Plugin,
    PluginResolveHookParam, PluginResolveHookResult,
};

pub struct FarmPluginResolve {
    root: String,
    // resolver: Resolver,
    external_config: RwLock<Option<ExternalConfig>>,
}
impl FarmPluginResolve {
    pub fn new(config: &Config) -> Self {
        Self {
            root: config.root.clone(),
            // resolver: Resolver::new(),
            external_config: RwLock::new(None),
        }
    }

    async fn is_external(&self, source: &str) -> bool {
        if let Some(external_config) = self.external_config.read().await.as_ref() {
            external_config.is_external(source)
        } else {
            false
        }
    }
    async fn try_alias(&self, source: &str) -> Option<String> {
        let alias = [("@/", "src/"), ("@", "src/"), ("~", "node_modules/")];
        for (key, value) in alias.iter() {
            if source.starts_with(key) {
                return Some(source.replacen(key, value, 1));
            }
        }
        None
    }

    async fn try_relative_or_absolute_path(&self, source: &str, base_dir: &Path) -> Option<String> {
        let path = if Path::new(source).is_absolute() {
            PathBuf::from(source)
        } else {
            base_dir.join(source)
        };

        if path.exists() {
            Some(path.to_string_lossy().into_owned())
        } else {
            None
        }
    }

    async fn try_node_modules(&self, source: &str, base_dir: &Path) -> Option<String> {
        let mut current = base_dir.to_path_buf();
        while let Some(parent) = current.parent() {
            let node_modules = current.join("node_modules").join(source);
            if node_modules.exists() {
                return Some(node_modules.to_string_lossy().into_owned());
            }
            current = parent.to_path_buf();
        }
        None
    }
}
#[async_trait]
impl Plugin for FarmPluginResolve {
    fn name(&self) -> &str {
        "FarmPluginResolve"
    }

    async fn resolve(
        &self,
        param: Arc<PluginResolveHookParam>,
        context: Arc<CompilationContext>,
    ) -> Result<Option<PluginResolveHookResult>> {
        let base_dir = if let Some(importer) = &param.importer {
            Path::new(&importer.resolved_path(&context.config.root))
                .parent()
                .unwrap()
                .to_path_buf()
        } else {
            PathBuf::from(&self.root)
        };

        // Check if it's external
        if self.is_external(&param.source).await {
            return Ok(Some(PluginResolveHookResult {
                resolved_path: param.source.clone(),
                external: true,
                side_effects: false,
                query: vec![],
                meta: HashMap::new(),
            }));
        }

        // Try resolving in order: alias, relative/absolute path, node_modules

        let resolved_path = self.try_alias(&param.source).await;
        let resolved_path = match resolved_path {
            Some(path) => Some(path),
            None => {
                self.try_relative_or_absolute_path(&param.source, &base_dir)
                    .await
            }
        };
        let resolved_path = match resolved_path {
            Some(path) => Some(path),
            None => self.try_node_modules(&param.source, &base_dir).await,
        };

        if let Some(resolved_path) = resolved_path {
            return Ok(Some(PluginResolveHookResult {
                resolved_path,
                external: false,
                side_effects: true, // Assume side effects by default
                query: vec![],      // You might want to parse query here
                meta: HashMap::new(),
            }));
        } else if context.config.resolve.auto_external_failed_resolve {
            return Ok(Some(PluginResolveHookResult {
                resolved_path: param.source.clone(),
                external: true,
                side_effects: false,
                query: vec![],
                meta: HashMap::new(),
            }));
        }

        Ok(None)
    }
}
