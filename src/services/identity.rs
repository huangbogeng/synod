use crate::{
    domain::{ValidationError, validate_handle},
    persistence::{BootstrapOutput, Database, StoreError},
};

#[derive(Debug, Clone)]
pub struct IdentityService {
    database: Database,
}

impl IdentityService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn bootstrap_human(
        &self,
        handle: &str,
        display_name: &str,
    ) -> Result<BootstrapOutput, IdentityError> {
        validate_handle(handle)?;
        let display_name = display_name.trim();
        if display_name.is_empty() || display_name.chars().count() > 100 {
            return Err(IdentityError::InvalidDisplayName);
        }

        self.database
            .bootstrap_human(handle, display_name, "bootstrap")
            .await
            .map_err(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("display name must contain 1-100 characters")]
    InvalidDisplayName,
    #[error(transparent)]
    Store(#[from] StoreError),
}
