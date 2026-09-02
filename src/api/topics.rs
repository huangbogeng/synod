use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{domain::TopicId, services::TopicService};

use super::{
    AppState,
    auth::{AuthenticatedPrincipal, Data},
    error::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct CreateTopicRequest {
    key: String,
    title: String,
    #[serde(default)]
    description: String,
}

pub async fn create(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Json(request): Json<CreateTopicRequest>,
) -> Result<(StatusCode, Json<Data<crate::domain::Topic>>), ApiError> {
    let topic = TopicService::new(state.database)
        .create(&principal, request.key, request.title, request.description)
        .await?;
    Ok((StatusCode::CREATED, Json(Data { data: topic })))
}

pub async fn list(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
) -> Result<Json<Data<Vec<crate::domain::Topic>>>, ApiError> {
    let topics = TopicService::new(state.database).list(&principal).await?;
    Ok(Json(Data { data: topics }))
}

pub async fn get(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(topic_id): Path<String>,
) -> Result<Json<Data<crate::domain::Topic>>, ApiError> {
    let topic_id = TopicId::from_str(&topic_id).map_err(|_| ApiError::NotFound)?;
    let topic = TopicService::new(state.database)
        .get(&principal, topic_id)
        .await?;
    Ok(Json(Data { data: topic }))
}
