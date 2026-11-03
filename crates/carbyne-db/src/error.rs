use std::fmt;

#[derive(Debug, Clone)]
pub enum DbError {
    InstanceNotFound,
    StorageError(String),
    SerializationError(String),
    ConfigError(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InstanceNotFound => write!(f, "instance not found"),
            Self::StorageError(e) => write!(f, "storage error: {}", e),
            Self::SerializationError(e) => write!(f, "serialization error: {}", e),
            Self::ConfigError(e) => write!(f, "config error: {}", e),
        }
    }
}

impl std::error::Error for DbError {}

pub type Result<T, E = DbError> = std::result::Result<T, E>;
