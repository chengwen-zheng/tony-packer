use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ModuleId;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
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