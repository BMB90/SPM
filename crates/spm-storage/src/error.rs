use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("connection pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("not found")]
    NotFound,
}

pub type StorageResult<T> = Result<T, StorageError>;

impl From<StorageError> for spm_core::SpmError {
    fn from(value: StorageError) -> Self {
        spm_core::SpmError::Storage(value.to_string())
    }
}
