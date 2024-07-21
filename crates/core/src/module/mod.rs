mod module_graph;
pub mod watch_graph;

use std::{
    any::Any,
    collections::HashSet,
    fmt::{self, Debug},
    path::Path,
    sync::Arc,
};

use downcast_rs::{impl_downcast, Downcast};
use heck::AsLowerCamelCase;
pub use module_graph::*;
use relative_path::RelativePath;
use rkyv::Deserialize;
use rkyv_dyn::archive_dyn;
use rkyv_typename::TypeName;
use swc_common::{comments::Comment, BytePos, DUMMY_SP};
use swc_css_ast::Stylesheet;
use swc_ecma_ast::Module as SwcModule;
use swc_html_ast::Document;
use toy_farm_macro_cache_item::cache_item;

use crate::deserialize;

#[cache_item]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
#[archive_attr(derive(Hash, Eq, PartialEq))]
pub struct ModuleId {
    relative_path: String,
    query_string: String,
}
pub const VIRTUAL_MODULE_PREFIX: &str = "virtual:";

impl ModuleId {
    pub fn new(relative_path: &str, query_string: &str) -> Self {
        Self {
            relative_path: relative_path.to_string(),
            query_string: query_string.to_string(),
        }
    }

    pub fn split_query(rp: &str) -> (String, String) {
        let mut parts = rp.split('?');
        let rp = parts.next().unwrap().to_string();
        let qs = parts.next().unwrap_or("").to_string();
        (rp, qs)
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn query_string(&self) -> &str {
        &self.query_string
    }

    /// transform the id back to resolved path
    pub fn resolved_path(&self, root: &str) -> String {
        // if self.relative_path is absolute path, return it directly
        if Path::new(self.relative_path()).is_absolute()
            || self.relative_path().starts_with(VIRTUAL_MODULE_PREFIX)
        {
            return self.relative_path().to_string();
        }

        RelativePath::new(self.relative_path())
            .to_logical_path(root)
            .to_string_lossy()
            .to_string()
    }
}

impl From<&str> for ModuleId {
    fn from(rp: &str) -> Self {
        let (rp, qs) = Self::split_query(rp);

        Self {
            relative_path: rp,
            query_string: qs,
        }
    }
}

impl From<String> for ModuleId {
    fn from(rp: String) -> Self {
        let (rp, qs) = Self::split_query(&rp);

        Self {
            relative_path: rp,
            query_string: qs,
        }
    }
}
impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.relative_path, self.query_string)
    }
}

impl<'de> serde::Deserialize<'de> for ModuleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <std::string::String as serde::Deserialize>::deserialize(deserializer)?;

        Ok(ModuleId::from(s))
    }
}

impl serde::Serialize for ModuleId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}
#[cache_item]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleType {
    // native supported module type by the core plugins
    Js,
    Jsx,
    Ts,
    Tsx,
    Css,
    Html,
    Asset,
    Runtime,
    // custom module type from using by custom plugins
    Custom(String),
}

impl serde::Serialize for ModuleType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ModuleType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <std::string::String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(s.into())
    }
}

impl<T: AsRef<str>> From<T> for ModuleType {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "js" => Self::Js,
            "jsx" => Self::Jsx,
            "ts" => Self::Ts,
            "tsx" => Self::Tsx,
            "css" => Self::Css,
            "html" => Self::Html,
            "asset" => Self::Asset,
            "runtime" => Self::Runtime,
            custom => Self::Custom(custom.to_string()),
        }
    }
}

impl fmt::Display for ModuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(s) => write!(f, "{}", s),
            _ => write!(f, "{}", AsLowerCamelCase(format!("{:?}", self))),
        }
    }
}

#[derive(Clone)]
#[cache_item]
pub struct Module {
    pub id: ModuleId,
    /// the type of this module, for example [ModuleType::Js]
    pub module_type: ModuleType,
    /// the module groups this module belongs to, used to construct [crate::module::module_group::ModuleGroupGraph]
    //   pub module_groups: HashSet<ModuleGroupId>,
    //   /// the resource pot this module belongs to
    //   pub resource_pot: Option<ResourcePotId>,
    //   /// the meta data of this module custom by plugins
    pub meta: Box<ModuleMetaData>,
    /// whether this module has side_effects
    pub side_effects: bool,
    /// the transformed source map chain of this module
    pub source_map_chain: Vec<Arc<String>>,
    /// whether this module marked as external
    pub external: bool,
    /// whether this module is immutable, for example, the module is immutable if it is from node_modules.
    /// This field will be set according to partialBundling.immutable of the user config, default to the module whose resolved_path contains ["/node_modules/"].
    pub immutable: bool,
    /// Execution order of this module in the module graph
    /// updated after the module graph is built
    pub execution_order: usize,
    /// Source size of this module
    pub size: usize,
    /// Source content after load and transform
    pub content: Arc<String>,
    /// Used exports of this module. Set by the tree-shake plugin
    pub used_exports: Vec<String>,
    /// last update timestamp
    pub last_update_timestamp: u128,
    /// content(after load and transform) hash
    pub content_hash: String,
    /// package name of this module
    pub package_name: String,
    /// package version of this module
    pub package_version: String,
}

impl Module {
    pub fn new(id: ModuleId) -> Self {
        Self {
            id,
            module_type: ModuleType::Custom("__farm_unknown".to_string()),
            meta: Box::new(ModuleMetaData::Custom(Box::new(EmptyModuleMetaData) as _)),
            // module_groups: HashSet::new(),
            // resource_pot: None,
            side_effects: true,
            source_map_chain: vec![],
            external: false,
            immutable: false,
            // default to the last
            execution_order: usize::MAX,
            size: 0,
            content: Arc::new("".to_string()),
            used_exports: vec![],
            last_update_timestamp: 0,
            content_hash: "".to_string(),
            package_name: "".to_string(),
            package_version: "".to_string(),
        }
    }
}

/// Script specific meta data, for example, [swc_ecma_ast::Module]
#[derive(Clone, Debug, PartialEq)]
#[cache_item]
pub struct ScriptModuleMetaData {
    pub ast: SwcModule,
    pub top_level_mark: u32,
    pub unresolved_mark: u32,
    pub module_system: ModuleSystem,
    /// true if this module calls `import.meta.hot.accept()` or `import.meta.hot.accept(mod => {})`
    pub hmr_self_accepted: bool,
    pub hmr_accepted_deps: HashSet<ModuleId>,
    pub comments: CommentsMetaData,
}

impl Default for ScriptModuleMetaData {
    fn default() -> Self {
        Self {
            ast: SwcModule {
                span: Default::default(),
                body: Default::default(),
                shebang: None,
            },
            top_level_mark: 0,
            unresolved_mark: 0,
            module_system: ModuleSystem::EsModule,
            hmr_self_accepted: false,
            hmr_accepted_deps: Default::default(),
            comments: Default::default(),
        }
    }
}

#[cache_item]
#[derive(Clone, Debug, PartialEq)]
pub struct CommentsMetaDataItem {
    pub byte_pos: BytePos,
    pub comment: Vec<Comment>,
}

#[cache_item]
#[derive(Clone, PartialEq, Debug, Default)]
pub struct CommentsMetaData {
    pub leading: Vec<CommentsMetaDataItem>,
    pub trailing: Vec<CommentsMetaDataItem>,
}
impl ScriptModuleMetaData {
    pub fn take_ast(&mut self) -> SwcModule {
        std::mem::replace(
            &mut self.ast,
            SwcModule {
                span: Default::default(),
                body: Default::default(),
                shebang: None,
            },
        )
    }

    pub fn set_ast(&mut self, ast: SwcModule) {
        self.ast = ast;
    }

    pub fn take_comments(&mut self) -> CommentsMetaData {
        std::mem::take(&mut self.comments)
    }

    pub fn set_comments(&mut self, comments: CommentsMetaData) {
        self.comments = comments;
    }
}

#[cache_item]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleSystem {
    EsModule,
    CommonJs,
    // Hybrid of commonjs and es-module
    Hybrid,
    Custom(String),
}

#[cache_item]
#[derive(Clone, Debug, PartialEq)]
pub struct CssModuleMetaData {
    pub ast: Stylesheet,
    pub comments: CommentsMetaData,
}

impl CssModuleMetaData {
    pub fn take_ast(&mut self) -> Stylesheet {
        std::mem::replace(
            &mut self.ast,
            Stylesheet {
                span: DUMMY_SP,
                rules: vec![],
            },
        )
    }

    pub fn set_ast(&mut self, ast: Stylesheet) {
        self.ast = ast;
    }
}

#[cache_item]
#[derive(Clone, PartialEq, Debug)]
pub struct HtmlModuleMetaData {
    pub ast: Document,
}

#[cache_item]
pub enum ModuleMetaData {
    Script(ScriptModuleMetaData),
    Css(CssModuleMetaData),
    Html(HtmlModuleMetaData),
    Custom(Box<dyn SerializeCustomModuleMetaData>),
}

impl Clone for ModuleMetaData {
    fn clone(&self) -> Self {
        match self {
            Self::Script(script) => Self::Script(script.clone()),
            Self::Css(css) => Self::Css(css.clone()),
            Self::Html(html) => Self::Html(html.clone()),
            Self::Custom(custom) => {
                let cloned_data = crate::serialize!(custom);
                let cloned_custom =
                    deserialize!(&cloned_data, Box<dyn SerializeCustomModuleMetaData>);
                Self::Custom(cloned_custom)
            }
        }
    }
}

impl fmt::Display for ModuleMetaData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Script(_) => write!(f, "Script"),
            Self::Css(_) => write!(f, "Css"),
            Self::Html(_) => write!(f, "Html"),
            Self::Custom(_) => write!(f, "Custom"),
        }
    }
}
impl ModuleMetaData {
    pub fn as_script_mut(&mut self) -> &mut ScriptModuleMetaData {
        if let Self::Script(script) = self {
            script
        } else {
            panic!("ModuleMetaData is not Script")
        }
    }

    pub fn as_script(&self) -> &ScriptModuleMetaData {
        if let Self::Script(script) = self {
            script
        } else {
            panic!("ModuleMetaData is not Script but {:?}", self.to_string())
        }
    }

    pub fn as_css(&self) -> &CssModuleMetaData {
        if let Self::Css(css) = self {
            css
        } else {
            panic!("ModuleMetaData is not css")
        }
    }

    pub fn as_css_mut(&mut self) -> &mut CssModuleMetaData {
        if let Self::Css(css) = self {
            css
        } else {
            panic!("ModuleMetaData is not css")
        }
    }

    pub fn as_html(&self) -> &HtmlModuleMetaData {
        if let Self::Html(html) = self {
            html
        } else {
            panic!("ModuleMetaData is not html")
        }
    }

    pub fn as_html_mut(&mut self) -> &mut HtmlModuleMetaData {
        if let Self::Html(html) = self {
            html
        } else {
            panic!("ModuleMetaData is not html")
        }
    }

    pub fn as_custom_mut<T: SerializeCustomModuleMetaData + 'static>(&mut self) -> &mut T {
        if let Self::Custom(custom) = self {
            if let Some(c) = custom.downcast_mut::<T>() {
                c
            } else {
                panic!("custom meta type is not serializable");
            }
        } else {
            panic!("ModuleMetaData is not Custom")
        }
    }

    pub fn as_custom<T: SerializeCustomModuleMetaData + 'static>(&self) -> &T {
        if let Self::Custom(custom) = self {
            if let Some(c) = custom.downcast_ref::<T>() {
                c
            } else {
                panic!("custom meta type is not serializable");
            }
        } else {
            panic!("ModuleMetaData is not Custom")
        }
    }
}
impl_downcast!(SerializeCustomModuleMetaData);

/// Trait that makes sure the trait object implements [rkyv::Serialize] and [rkyv::Deserialize]
#[archive_dyn(deserialize)]
pub trait CustomModuleMetaData: Any + Send + Sync + Downcast {}

/// initial empty custom data, plugins may replace this
#[derive(Clone, Debug, PartialEq)]
#[cache_item(CustomModuleMetaData)]
pub struct EmptyModuleMetaData;
