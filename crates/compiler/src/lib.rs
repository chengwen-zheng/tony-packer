use std::sync::Arc;

use toy_farm_core::{CompilationContext, Config};

pub mod build;

pub struct Compiler {
    context: Arc<CompilationContext>,
}

impl Compiler {
    pub fn new(config: Config) -> Compiler {
        Compiler {
            context: Arc::new(CompilationContext::new(config)),
        }
    }

    pub fn compile(&self) {
        todo!();
    }
}
