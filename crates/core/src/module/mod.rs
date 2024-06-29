mod module_graph;

pub use module_graph::*;

#[derive(Eq, Hash, PartialEq, Debug, Clone, PartialOrd, Ord)]
pub struct ModuleId {
    relative_path: String,
    query_string: String,
}

impl ModuleId {
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

impl From<&str> for ModuleId {
    fn from(rp: &str) -> Self {
        let (rp, qs) = Self::split_query(rp);

        Self {
            relative_path: rp,
            query_string: qs,
        }
    }
}

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
pub struct Module {
    pub id: ModuleId,
    /// the type of this module, for example [ModuleType::Js]
    pub module_type: ModuleType,
}

impl Module {
    pub fn new(id: ModuleId) -> Self {
        Self {
            id,
            module_type: ModuleType::Custom("__farm_unknown".to_string()),
        }
    }
}
