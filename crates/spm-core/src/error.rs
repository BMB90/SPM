use thiserror::Error;

/// Errors surfaced by collectors and the core engine.
#[derive(Debug, Error)]
pub enum SpmError {
    #[error("collector '{collector}' failed: {message}")]
    Collector { collector: String, message: String },

    #[error("collector '{collector}' is not available on this system: {reason}")]
    Unavailable { collector: String, reason: String },

    #[error("permission denied while running collector '{collector}': {message}")]
    PermissionDenied { collector: String, message: String },

    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type SpmResult<T> = Result<T, SpmError>;
