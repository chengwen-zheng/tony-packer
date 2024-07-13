mod module_graph;

use std::{fmt, sync::Arc};

pub use module_graph::*;
use serde::{Deserialize, Serialize};
use toy_farm_macro_cache_item::cache_item;

#[cache_item]
#[derive(Eq, Hash, PartialEq, Debug, Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleId {
    relative_path: String,
    query_string: String,
}

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
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.query_string.is_empty() {
            write!(f, "{}", self.relative_path)
        } else {
            write!(f, "{}?{}", self.relative_path, self.query_string)
        }
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

#[derive(Debug, Clone)]
#[cache_item]
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
    //   pub meta: Box<ModuleMetaData>,
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
            // meta: Box::new(ModuleMetaData::Custom(Box::new(EmptyModuleMetaData) as _)),
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
