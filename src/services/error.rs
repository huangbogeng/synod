use crate::{domain::ValidationError, persistence::StoreError};

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("invalid reference: {0}")]
    InvalidReference(&'static str),
    #[error("operation is not permitted")]
    Forbidden,
    #[error("resource already exists")]
    Conflict,
    #[error("resource was not found")]
    NotFound,
    #[error("storage operation failed")]
    Storage(#[source] sqlx::Error),
    #[error("stored data is invalid")]
    CorruptData,
}

impl From<StoreError> for ServiceError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Conflict => Self::Conflict,
            StoreError::NotFound => Self::NotFound,
            StoreError::PermissionDenied => Self::Forbidden,
            StoreError::InvalidReference(message) => Self::InvalidReference(message),
            StoreError::Sqlx(error) => Self::Storage(error),
            StoreError::CorruptData(_) | StoreError::AlreadyBootstrapped => Self::CorruptData,
        }
    }
}
