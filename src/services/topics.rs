use crate::{
    domain::{CreateTopic, Principal, PrincipalKind, Topic, TopicId, ValidationError},
    persistence::{Database, StoreError},
};

#[derive(Debug, Clone)]
pub struct TopicService {
    database: Database,
}

impl TopicService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn create(
        &self,
        actor: &Principal,
        key: String,
        title: String,
        description: String,
    ) -> Result<Topic, ServiceError> {
        if actor.kind != PrincipalKind::Human {
            return Err(ServiceError::Forbidden);
        }
        let input = CreateTopic::new(key, title, description)?;
        self.database
            .insert_topic(actor, &input)
            .await
            .map_err(Into::into)
    }

    pub async fn list(&self, actor: &Principal) -> Result<Vec<Topic>, ServiceError> {
        self.database
            .list_topics_for(actor.id)
            .await
            .map_err(Into::into)
    }

    pub async fn get(&self, actor: &Principal, topic_id: TopicId) -> Result<Topic, ServiceError> {
        self.database
            .get_topic_for(actor.id, topic_id)
            .await
            .map_err(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
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
            StoreError::Sqlx(error) => Self::Storage(error),
            StoreError::CorruptData(_) | StoreError::AlreadyBootstrapped => Self::CorruptData,
        }
    }
}
