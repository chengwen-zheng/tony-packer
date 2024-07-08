use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilationError {}

pub type Result<T> = core::result::Result<T, CompilationError>;
