use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toy_farm_macro_cache_item::cache_item;

pub mod plugin_driver;

use crate::{error::Result, CompilationContext, Config, ModuleId, ModuleMetaData, ModuleType};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cache_item]
pub enum ResolveKind {
    /// entry input in the config
    Entry(String),
    /// static import, e.g. `import a from './a'`
    #[default]
    Import,
    /// static export, e.g. `export * from './a'`
    ExportFrom,
    /// dynamic import, e.g. `import('./a').then(module => console.log(module))`
    DynamicImport,
    /// cjs require, e.g. `require('./a')`
    Require,
    /// @import of css, e.g. @import './a.css'
    CssAtImport,
    /// url() of css, e.g. url('./a.png')
    CssUrl,
    /// `<script src="./index.html" />` of html
    ScriptSrc,
    /// `<link href="index.css" />` of html
    LinkHref,
    /// Hmr update
    HmrUpdate,
    /// Custom ResolveKind, e.g. `const worker = new Worker(new Url("worker.js"))` of a web worker
    Custom(String),
}
impl From<&str> for ResolveKind {
    fn from(value: &str) -> Self {
        serde_json::from_str(value).unwrap()
    }
}

impl From<ResolveKind> for String {
    fn from(value: ResolveKind) -> Self {
        serde_json::to_string(&value).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginHookContext {
    /// if this hook is called by the compiler, its value is [None]
    /// if this hook is called by other plugins, its value is set by the caller plugins.
    pub caller: Option<String>,
    /// meta data passed between plugins
    pub meta: HashMap<String, String>,
}

// MARK: - resolve
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginResolveHookResult {
    /// resolved path, normally a absolute file path.
    pub resolved_path: String,
    /// whether this module should be external, if true, the module won't present in the final result
    pub external: bool,
    /// whether this module has side effects, affects tree shaking. By default, it's true, means all modules may has side effects.
    /// use sideEffects field in package.json to mark it as side effects free
    pub side_effects: bool,
    /// the query parsed from specifier, for example, query should be `{ inline: "" }` if specifier is `./a.png?inline`
    /// if you custom plugins, your plugin should be responsible for parsing query
    /// if you just want a normal query parsing like the example above, [farmfe_toolkit::resolve::parse_query] should be helpful
    pub query: Vec<(String, String)>,
    #[doc = r"the meta data passed between plugins and hooks"]
    pub meta: HashMap<String, String>,
}

impl Default for PluginResolveHookResult {
    fn default() -> Self {
        Self {
            side_effects: true,
            resolved_path: "unknown".to_string(),
            external: false,
            query: vec![],
            meta: Default::default(),
        }
    }
}

/// Parameter of the resolve hook
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginResolveHookParam {
    /// the source would like to resolve, for example, './index'
    pub source: String,
    /// the start location to resolve `specifier`, being [None] if resolving a entry or resolving a hmr update.
    pub importer: Option<ModuleId>,
    /// for example, [ResolveKind::Import] for static import (`import a from './a'`)
    pub kind: ResolveKind,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cache_item]
pub struct PluginAnalyzeDepsHookResultEntry {
    pub source: String,
    pub kind: ResolveKind,
}

// MARK: - load
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginLoadHookParam {
    /// the module id string
    pub module_id: String,
    /// the resolved path from resolve hook
    pub resolved_path: String,
    /// the query map
    pub query: Vec<(String, String)>,
    /// the meta data passed between plugins and hooks
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PluginLoadHookResult {
    /// the source content of the module
    pub content: String,
    /// the type of the module, for example [ModuleType::Js] stands for a normal javascript file,
    /// usually end with `.js` extension
    pub module_type: ModuleType,
    /// source map of the module
    pub source_map: Option<String>,
}

// MARK: - transform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTransformHookParam {
    /// the module id string
    pub module_id: String,
    /// source content after load or transformed result of previous plugin
    pub content: String,
    /// module type after load
    pub module_type: ModuleType,
    /// resolved path from resolve hook
    pub resolved_path: String,
    /// query from resolve hook
    pub query: Vec<(String, String)>,
    /// the meta data passed between plugins and hooks
    pub meta: HashMap<String, String>,
    /// source map chain of previous plugins
    pub source_map_chain: Vec<Arc<String>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginTransformHookResult {
    /// transformed source content, will be passed to next plugin.
    pub content: String,
    /// you can change the module type after transform.
    pub module_type: Option<ModuleType>,
    /// transformed source map, all plugins' transformed source map will be stored as a source map chain.
    pub source_map: Option<String>,
    /// if true, the previous source map chain will be ignored, and the source map chain will be reset to [source_map] returned by this plugin.
    pub ignore_previous_source_map: bool,
}

// MARK: - PARSE
#[derive(Debug, Clone)]
pub struct PluginParseHookParam {
    /// module id
    pub module_id: ModuleId,
    /// resolved path
    pub resolved_path: String,
    /// resolved query
    pub query: Vec<(String, String)>,
    pub module_type: ModuleType,
    /// source content(after transform)
    pub content: Arc<String>,
}

pub struct PluginProcessModuleHookParam<'a> {
    pub module_id: &'a ModuleId,
    pub module_type: &'a ModuleType,
    pub content: Arc<String>,
    pub meta: &'a mut ModuleMetaData,
}

pub const DEFAULT_PRIORITY: i32 = 100;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    fn priority(&self) -> i32 {
        DEFAULT_PRIORITY
    }

    async fn config(&self, _config: &mut Config) -> Result<Option<()>> {
        Ok(None)
    }

    async fn resolve(
        &self,
        _param: Arc<PluginResolveHookParam>,
        _context: Arc<CompilationContext>,
    ) -> Result<Option<PluginResolveHookResult>> {
        Ok(None)
    }

    async fn load(
        &self,
        _param: Arc<PluginLoadHookParam>,
        _context: Arc<CompilationContext>,
    ) -> Result<Option<PluginLoadHookResult>> {
        Ok(None)
    }

    async fn transform(
        &self,
        _param: PluginTransformHookParam,
        _context: Arc<CompilationContext>,
    ) -> Result<Option<PluginTransformHookResult>> {
        Ok(None)
    }

    async fn parse(
        &self,
        _param: Arc<PluginParseHookParam>,
        _context: Arc<CompilationContext>,
    ) -> Result<Option<ModuleMetaData>> {
        Ok(None)
    }

    async fn process_module(
        &self,
        _param: &mut PluginProcessModuleHookParam,
        _context: &Arc<CompilationContext>,
    ) -> Result<Option<ModuleMetaData>> {
        Ok(None)
    }
}
