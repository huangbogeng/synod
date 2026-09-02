use crate::{
    domain::{
        AiMember, Model, ModelId, ModelInput, Principal, Provider, ProviderAdapter, ProviderId,
        validate_handle,
    },
    persistence::Database,
};

use super::ServiceError;

#[derive(Debug, Clone)]
pub struct AdminService {
    database: Database,
}

impl AdminService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn create_provider(
        &self,
        actor: &Principal,
        name: String,
        adapter: ProviderAdapter,
        base_url: String,
        credential_ref: String,
    ) -> Result<Provider, ServiceError> {
        validate_text(&name, 100, "provider name is invalid")?;
        validate_text(&base_url, 2_000, "provider base URL is invalid")?;
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err(ServiceError::InvalidReference(
                "provider base URL is invalid",
            ));
        }
        validate_text(&credential_ref, 500, "credential reference is invalid")?;
        if !(credential_ref.starts_with("env://") || credential_ref.starts_with("secret://")) {
            return Err(ServiceError::InvalidReference(
                "credential reference must use env:// or secret://",
            ));
        }
        self.database
            .insert_provider(actor.id, name.trim(), adapter, &base_url, &credential_ref)
            .await
            .map_err(Into::into)
    }

    pub async fn list_providers(&self, actor: &Principal) -> Result<Vec<Provider>, ServiceError> {
        self.database
            .list_providers(actor.id)
            .await
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_model(
        &self,
        actor: &Principal,
        provider_id: ProviderId,
        model_name: String,
        display_name: String,
        capabilities: serde_json::Value,
        limits: serde_json::Value,
        defaults: serde_json::Value,
    ) -> Result<Model, ServiceError> {
        validate_text(&model_name, 200, "model name is invalid")?;
        validate_text(&display_name, 100, "model display name is invalid")?;
        for value in [&capabilities, &limits, &defaults] {
            if !value.is_object() || value.to_string().len() > 20_000 {
                return Err(ServiceError::InvalidReference(
                    "model configuration must be an object no larger than 20000 bytes",
                ));
            }
        }
        let input = ModelInput {
            provider_id,
            model_name: model_name.trim().to_owned(),
            display_name: display_name.trim().to_owned(),
            capabilities,
            limits,
            defaults,
        };
        self.database
            .insert_model(actor.id, &input)
            .await
            .map_err(Into::into)
    }

    pub async fn list_models(&self, actor: &Principal) -> Result<Vec<Model>, ServiceError> {
        self.database
            .list_models(actor.id)
            .await
            .map_err(Into::into)
    }

    pub async fn create_ai_member(
        &self,
        actor: &Principal,
        handle: String,
        display_name: String,
        identity_prompt: String,
        default_model_id: ModelId,
    ) -> Result<AiMember, ServiceError> {
        validate_handle(&handle)?;
        validate_text(&display_name, 100, "AI Member display name is invalid")?;
        validate_text(&identity_prompt, 100_000, "identity Prompt is invalid")?;
        self.database
            .insert_ai_member(
                actor.id,
                &handle,
                display_name.trim(),
                &identity_prompt,
                default_model_id,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn list_ai_members(&self, actor: &Principal) -> Result<Vec<AiMember>, ServiceError> {
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
