use std::str::FromStr;

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::{ModelId, ProviderAdapter, ProviderId},
    providers::HttpGateway,
    services::AdminService,
};

use super::{
    AppState,
    auth::{AuthenticatedPrincipal, Data},
    error::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    name: String,
    adapter: ProviderAdapter,
    base_url: String,
    credential_ref: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    provider_id: String,
    model_name: String,
    display_name: String,
    #[serde(default = "empty_object")]
    capabilities: serde_json::Value,
    #[serde(default = "empty_object")]
    limits: serde_json::Value,
    #[serde(default = "empty_object")]
    defaults: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateAiMemberRequest {
    handle: String,
    display_name: String,
    identity_prompt: String,
    default_model_id: Option<String>,
    provider_id: Option<String>,
    model_name: Option<String>,
}

pub async fn create_provider(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Json(request): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<Data<crate::domain::Provider>>), ApiError> {
    let service = AdminService::new(state.database);
    let provider = match (request.credential_ref, request.api_key) {
        (Some(reference), None) => {
            service
                .create_provider(
                    &principal,
                    request.name,
                    request.adapter,
                    request.base_url,
                    reference,
                )
                .await?
        }
        (None, Some(secret)) => {
            service
                .create_provider_with_secret(
                    &principal,
                    request.name,
                    request.adapter,
                    request.base_url,
                    secret,
                )
                .await?
        }
        _ => {
            return Err(ApiError::BadRequest(
                "provide exactly one of credential_ref or api_key",
            ));
        }
    };
    Ok((StatusCode::CREATED, Json(Data { data: provider })))
}

pub async fn list_providers(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
) -> Result<Json<Data<Vec<crate::domain::Provider>>>, ApiError> {
    let providers = AdminService::new(state.database)
        .list_providers(&principal)
        .await?;
    Ok(Json(Data { data: providers }))
}

pub async fn discover_models(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
) -> Result<Json<Data<Vec<crate::providers::DiscoveredModel>>>, ApiError> {
    let provider_id = ProviderId::from_str(&provider_id)
        .map_err(|_| ApiError::BadRequest("provider identifier is invalid"))?;
    let (base_url, credential_ref) = AdminService::new(state.database.clone())
        .provider_connection(&principal, provider_id)
        .await?;
    let gateway = HttpGateway::new(state.database).map_err(|error| {
        tracing::error!(%error, "provider discovery client setup failed");
        ApiError::Internal
    })?;
    let models = gateway
        .discover_models(&base_url, &credential_ref)
        .await
        .map_err(|error| {
            tracing::warn!(%provider_id, %error, "provider model discovery failed");
            ApiError::ProviderUnavailable
        })?;
    Ok(Json(Data { data: models }))
}

pub async fn create_model(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Json(request): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<Data<crate::domain::Model>>), ApiError> {
    let provider_id = ProviderId::from_str(&request.provider_id)
        .map_err(|_| ApiError::BadRequest("provider identifier is invalid"))?;
    let model = AdminService::new(state.database)
        .create_model(
            &principal,
            provider_id,
            request.model_name,
            request.display_name,
            request.capabilities,
            request.limits,
            request.defaults,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data { data: model })))
}

pub async fn list_models(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
) -> Result<Json<Data<Vec<crate::domain::Model>>>, ApiError> {
    let models = AdminService::new(state.database)
        .list_models(&principal)
        .await?;
    Ok(Json(Data { data: models }))
}

pub async fn create_ai_member(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Json(request): Json<CreateAiMemberRequest>,
) -> Result<(StatusCode, Json<Data<crate::domain::AiMember>>), ApiError> {
    let service = AdminService::new(state.database);
    let member = match (
        request.default_model_id,
        request.provider_id,
        request.model_name,
    ) {
        (Some(model_id), None, None) => {
            let model_id = ModelId::from_str(&model_id)
                .map_err(|_| ApiError::BadRequest("model identifier is invalid"))?;
            service
                .create_ai_member(
                    &principal,
                    request.handle,
                    request.display_name,
                    request.identity_prompt,
                    model_id,
                )
                .await?
        }
        (None, Some(provider_id), Some(model_name)) => {
            let provider_id = ProviderId::from_str(&provider_id)
                .map_err(|_| ApiError::BadRequest("provider identifier is invalid"))?;
            service
                .create_ai_member_for_model(
                    &principal,
                    request.handle,
                    request.display_name,
                    request.identity_prompt,
                    provider_id,
                    model_name,
                )
                .await?
        }
        _ => {
            return Err(ApiError::BadRequest(
                "provide either default_model_id or provider_id with model_name",
            ));
        }
    };
    Ok((StatusCode::CREATED, Json(Data { data: member })))
}

pub async fn list_ai_members(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
) -> Result<Json<Data<Vec<crate::domain::AiMember>>>, ApiError> {
    let members = AdminService::new(state.database)
        .list_ai_members(&principal)
        .await?;
    Ok(Json(Data { data: members }))
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}
