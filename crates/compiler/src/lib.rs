use std::{sync::Arc, vec};

use toy_farm_core::{CompilationContext, Config};
use toy_farm_plugin_resolve::FarmPluginResolve;

pub mod build;

pub struct Compiler {
    context: Arc<CompilationContext>,
}

impl Compiler {
    pub async fn new(config: Config) -> Compiler {
        let plugins = vec![Arc::new(FarmPluginResolve::new(&config)) as _];

        let mut context = CompilationContext::new(config, plugins);
        let _ = context.plugin_driver.config(&mut context.config).await;
        Compiler {
            context: Arc::new(context),
        }
    }

    pub async fn compile(&self) {
        self.build().await;
    }
}
