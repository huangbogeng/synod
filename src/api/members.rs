use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    domain::{MembershipRole, PrincipalId, TeamId, TopicId},
    services::MembershipService,
};

use super::{
    AppState,
    auth::{AuthenticatedPrincipal, Data},
    error::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct PutMemberRequest {
    role: MembershipRole,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    handle: String,
    display_name: String,
}

pub async fn put_topic_member(
    State(state): State<AppState>,
    AuthenticatedPrincipal(actor): AuthenticatedPrincipal,
    Path((topic_id, principal_id)): Path<(String, String)>,
    Json(request): Json<PutMemberRequest>,
) -> Result<Json<Data<crate::domain::TopicMember>>, ApiError> {
    let member = MembershipService::new(state.database)
        .put_topic_member(
            &actor,
            parse_id(&topic_id, "topic identifier is invalid")?,
            parse_id(&principal_id, "principal identifier is invalid")?,
            request.role,
        )
        .await?;
    Ok(Json(Data { data: member }))
}

pub async fn list_topic_members(
    State(state): State<AppState>,
    AuthenticatedPrincipal(actor): AuthenticatedPrincipal,
    Path(topic_id): Path<String>,
) -> Result<Json<Data<Vec<crate::domain::TopicMember>>>, ApiError> {
    let members = MembershipService::new(state.database)
        .list_topic_members(
            &actor,
            parse_id::<TopicId>(&topic_id, "topic identifier is invalid")?,
        )
        .await?;
    Ok(Json(Data { data: members }))
}

pub async fn create_team(
    State(state): State<AppState>,
    AuthenticatedPrincipal(actor): AuthenticatedPrincipal,
    Path(topic_id): Path<String>,
    Json(request): Json<CreateTeamRequest>,
) -> Result<(StatusCode, Json<Data<crate::domain::Team>>), ApiError> {
    let team = MembershipService::new(state.database)
        .create_team(
            &actor,
            parse_id::<TopicId>(&topic_id, "topic identifier is invalid")?,
            request.handle,
            request.display_name,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data { data: team })))
}

pub async fn list_teams(
    State(state): State<AppState>,
    AuthenticatedPrincipal(actor): AuthenticatedPrincipal,
    Path(topic_id): Path<String>,
) -> Result<Json<Data<Vec<crate::domain::Team>>>, ApiError> {
    let teams = MembershipService::new(state.database)
        .list_teams(
            &actor,
            parse_id::<TopicId>(&topic_id, "topic identifier is invalid")?,
        )
        .await?;
    Ok(Json(Data { data: teams }))
}

pub async fn put_team_member(
    State(state): State<AppState>,
    AuthenticatedPrincipal(actor): AuthenticatedPrincipal,
    Path((team_id, principal_id)): Path<(String, String)>,
) -> Result<Json<Data<crate::domain::Team>>, ApiError> {
    let team = MembershipService::new(state.database)
        .put_team_member(
            &actor,
            parse_id::<TeamId>(&team_id, "team identifier is invalid")?,
            parse_id::<PrincipalId>(&principal_id, "principal identifier is invalid")?,
        )
        .await?;
    Ok(Json(Data { data: team }))
}

fn parse_id<T>(value: &str, message: &'static str) -> Result<T, ApiError>
where
    T: FromStr,
{
    value.parse().map_err(|_| ApiError::BadRequest(message))
}
