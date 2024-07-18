use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum CompilationError {
    JoinError(JoinError),
    ResolveError(String),
}

pub type Result<T> = core::result::Result<T, CompilationError>;

// please help impl form JoinError to CompilationError
impl From<JoinError> for CompilationError {
    fn from(e: JoinError) -> Self {
        CompilationError::JoinError(e)
    }
}

// please help impl std::fmt::Display for CompilationError
impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilationError::JoinError(e) => write!(f, "JoinError: {}", e),
            CompilationError::ResolveError(e) => write!(f, "ResolveError: {}", e),
        }
    }
}
