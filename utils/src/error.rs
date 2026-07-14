use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Path does not exist: '{0}'")]
    PathNotFound(String),
    #[error("Path is not a regular file: '{0}'")]
    NotAFile(String),
    #[error("Path is not readable or permission denied: '{0}'")]
    NotReadable(String),
    #[error("Empty file encountered: '{0}'")]
    EmptyFile(String),
    #[error("Invalid file signature for '{path}': expected {expected}")]
    InvalidSignature { path: String, expected: String },
    #[error("Invalid metadata filter key format: '{0}'")]
    InvalidFilterKey(String),
}
