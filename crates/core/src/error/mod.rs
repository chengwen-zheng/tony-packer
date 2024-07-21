use std::error::Error;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("JoinError: {0}")]
    JoinError(JoinError),
    #[error("Can not resolve `{src}` from {importer}.\nOriginal error: {source:?}.\n\nPotential Causes:\n1.The file that `{src}` points to does not exist.\n2.Install it first if `{src}` is an dependency from node_modules, if you are using pnpm refer to [https://pnpm.io/faq#pnpm-does-not-work-with-your-project-here] for solutions.\n3. If `{src}` is a alias, make sure your alias config is correct.\n")]
    ResolveError {
        importer: String,
        src: String,
        #[source]
        source: Option<Box<dyn Error + Send + Sync>>,
    },

    #[error("Can not load `{resolved_path}`. Original error: \n{source:?}.\n\nPotential Causes:\n1.This kind of module is not supported, you may need plugins to support it.\n")]
    LoadError {
        resolved_path: String,
        #[source]
        source: Option<Box<dyn Error + Send + Sync>>,
    },

    #[error("{0}")]
    GenericError(String),
}

pub type Result<T> = core::result::Result<T, CompilationError>;

// please help impl form JoinError to CompilationError
impl From<JoinError> for CompilationError {
    fn from(e: JoinError) -> Self {
        CompilationError::JoinError(e)
    }
}
