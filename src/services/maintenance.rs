use crate::{
    domain::{AiMember, validate_handle},
    persistence::Database,
};

use super::ServiceError;

#[derive(Debug, Clone)]
pub struct MaintenanceService {
    database: Database,
}

impl MaintenanceService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn clear_all_topics(&self) -> Result<u64, ServiceError> {
        let actor = self.database.local_bootstrap_principal().await?;
        self.database
            .clear_all_topics_local(actor.id)
            .await
            .map_err(Into::into)
    }

    pub async fn rotate_bootstrap_token(&self) -> Result<String, ServiceError> {
        let actor = self.database.local_bootstrap_principal().await?;
        self.database
            .rotate_bootstrap_token_local(actor.id)
            .await
            .map_err(Into::into)
    }

    pub async fn count_topics(&self) -> Result<u64, ServiceError> {
        let actor = self.database.local_bootstrap_principal().await?;
        let count = self.database.list_topics_for(actor.id).await?.len();
        u64::try_from(count).map_err(|_| ServiceError::InvalidReference("Topic count overflow"))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn configure_ai_member(
        &self,
        handle: String,
        display_name: String,
        identity_prompt: String,
        provider_name: String,
        model_name: String,
        execution_defaults: serde_json::Value,
    ) -> Result<AiMember, ServiceError> {
        validate_handle(&handle)?;
        validate_text(&display_name, 100, "AI Member display name is invalid")?;
        validate_text(&identity_prompt, 100_000, "identity Prompt is invalid")?;
        validate_text(&provider_name, 100, "Provider name is invalid")?;
        validate_text(&model_name, 200, "model name is invalid")?;
        if !execution_defaults.is_object() || execution_defaults.to_string().len() > 20_000 {
            return Err(ServiceError::InvalidReference(
                "AI Member execution defaults must be an object no larger than 20000 bytes",
            ));
        }
        let actor = self.database.local_bootstrap_principal().await?;
        self.database
            .configure_ai_member_local(
                actor.id,
                &handle,
                display_name.trim(),
                &identity_prompt,
                provider_name.trim(),
                model_name.trim(),
                &execution_defaults,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn list_ai_members(&self) -> Result<Vec<AiMember>, ServiceError> {
        let actor = self.database.local_bootstrap_principal().await?;
        self.database
            .list_ai_members(actor.id)
            .await
            .map_err(Into::into)
    }
}

fn validate_text(value: &str, max_chars: usize, message: &'static str) -> Result<(), ServiceError> {
    if value.trim().is_empty() || value.chars().count() > max_chars {
        Err(ServiceError::InvalidReference(message))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        domain::ProviderAdapter,
        persistence::Database,
        services::{AdminService, TopicService},
    };

    use super::MaintenanceService;

    #[tokio::test]
    async fn local_configuration_is_idempotent_and_clear_is_scoped_to_topics() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let human = database
            .bootstrap_human("admin", "Administrator", "test")
            .await
            .unwrap()
            .principal;
        TopicService::new(database.clone())
            .create(
                &human,
                "synod".to_owned(),
                "Synod".to_owned(),
                "Development".to_owned(),
            )
            .await
            .unwrap();
        let provider = AdminService::new(database.clone())
            .create_provider(
                &human,
                "MiniMax".to_owned(),
                ProviderAdapter::OpenaiCompatible,
                "https://api.minimaxi.com/v1".to_owned(),
                "env://MINIMAX_API_KEY".to_owned(),
            )
            .await
            .unwrap();
        AdminService::new(database.clone())
            .create_model(
                &human,
                provider.id,
                "MiniMax-M3".to_owned(),
                "MiniMax M3".to_owned(),
                serde_json::json!({}),
                serde_json::json!({}),
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let maintenance = MaintenanceService::new(database.clone());
        let first = maintenance
            .configure_ai_member(
                "developer".to_owned(),
                "Developer".to_owned(),
                "Review code.".to_owned(),
                "MiniMax".to_owned(),
                "MiniMax-M3".to_owned(),
                serde_json::json!({"temperature": 0.2}),
            )
            .await
            .unwrap();
        let updated = maintenance
            .configure_ai_member(
                "developer".to_owned(),
                "Developer · Exploratory".to_owned(),
                "Review code and alternatives.".to_owned(),
                "MiniMax".to_owned(),
                "MiniMax-M3".to_owned(),
                serde_json::json!({"temperature": 0.9}),
            )
            .await
            .unwrap();
        assert_eq!(updated.principal.id, first.principal.id);
        assert_eq!(updated.identity_prompt_version, 2);
        assert_eq!(updated.execution_defaults["temperature"], 0.9);
        assert_eq!(maintenance.list_ai_members().await.unwrap().len(), 1);

        assert_eq!(maintenance.clear_all_topics().await.unwrap(), 1);
        assert!(
            TopicService::new(database.clone())
                .list(&human)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(maintenance.list_ai_members().await.unwrap().len(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM providers")
                .fetch_one(database.pool())
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn rotating_bootstrap_token_revokes_the_previous_token() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let bootstrap = database
            .bootstrap_human("admin", "Administrator", "test")
            .await
            .unwrap();
        let principal_id = bootstrap.principal.id;

        let replacement = MaintenanceService::new(database.clone())
            .rotate_bootstrap_token()
            .await
            .unwrap();

        assert!(
            database
                .authenticate(&bootstrap.token)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            database
                .authenticate(&replacement)
                .await
                .unwrap()
                .unwrap()
                .id,
            principal_id
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM principal_tokens
                 WHERE principal_id = ? AND revoked_at IS NULL"
            )
            .bind(principal_id.to_string())
            .fetch_one(database.pool())
            .await
            .unwrap(),
            1
        );
    }
}
