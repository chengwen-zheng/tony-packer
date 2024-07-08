use tokio::sync::RwLock;

use crate::{Config, ModuleGraph};

pub struct CompilationContext {
    pub module_graph: Box<RwLock<ModuleGraph>>,
    pub config: Config,
}

impl CompilationContext {
    pub fn new(config: Config) -> CompilationContext {
        CompilationContext {
            module_graph: Box::new(RwLock::new(ModuleGraph::new())),
            config,
        }
    }
}
