use std::str::FromStr;

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    domain::{ModelId, ProviderAdapter, ProviderId},
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
    credential_ref: String,
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
    default_model_id: String,
}

pub async fn create_provider(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Json(request): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<Data<crate::domain::Provider>>), ApiError> {
    let provider = AdminService::new(state.database)
        .create_provider(
            &principal,
            request.name,
            request.adapter,
            request.base_url,
            request.credential_ref,
        )
        .await?;
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
    let model_id = ModelId::from_str(&request.default_model_id)
        .map_err(|_| ApiError::BadRequest("model identifier is invalid"))?;
    let member = AdminService::new(state.database)
        .create_ai_member(
            &principal,
            request.handle,
            request.display_name,
            request.identity_prompt,
            model_id,
        )
        .await?;
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
