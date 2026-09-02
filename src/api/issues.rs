use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{CommentId, CommentKind, DispatchId, TopicId, TopicItemId},
    persistence::StoredCreation,
    services::IssueService,
};

use super::{
    AppState,
    auth::{AuthenticatedPrincipal, Data},
    error::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct CreateIssueRequest {
    issue_type: String,
    title: String,
    #[serde(default)]
    body: String,
    parent_issue_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    #[serde(default = "default_comment_kind")]
    kind: CommentKind,
    body: String,
    reply_to_comment_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreationResponse<T> {
    data: T,
    dispatch: Option<DispatchResponse>,
}

#[derive(Debug, Serialize)]
struct DispatchResponse {
    id: DispatchId,
    status: &'static str,
}

pub async fn list_types(
    State(state): State<AppState>,
    _principal: AuthenticatedPrincipal,
) -> Result<Json<Data<Vec<crate::domain::IssueType>>>, ApiError> {
    let types = IssueService::new(state.database).list_types().await?;
    Ok(Json(Data { data: types }))
}

pub async fn create(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(topic_id): Path<String>,
    Json(request): Json<CreateIssueRequest>,
) -> Result<(StatusCode, Json<CreationResponse<crate::domain::Issue>>), ApiError> {
    let topic_id = parse_id::<TopicId>(&topic_id)?;
    let parent_issue_id = request
        .parent_issue_id
        .map(|value| parse_id::<TopicItemId>(&value))
        .transpose()?;
    let creation = IssueService::new(state.database)
        .create_issue(
            &principal,
            topic_id,
            request.issue_type,
            request.title,
            request.body,
            parent_issue_id,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(creation_response(creation))))
}

pub async fn list(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(topic_id): Path<String>,
) -> Result<Json<Data<Vec<crate::domain::Issue>>>, ApiError> {
    let topic_id = parse_id::<TopicId>(&topic_id)?;
    let issues = IssueService::new(state.database)
        .list_issues(&principal, topic_id)
        .await?;
    Ok(Json(Data { data: issues }))
}

pub async fn get(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(issue_id): Path<String>,
) -> Result<Json<Data<crate::domain::Issue>>, ApiError> {
    let issue_id = parse_id::<TopicItemId>(&issue_id)?;
    let issue = IssueService::new(state.database)
        .get_issue(&principal, issue_id)
        .await?;
    Ok(Json(Data { data: issue }))
}

pub async fn create_comment(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(issue_id): Path<String>,
    Json(request): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CreationResponse<crate::domain::Comment>>), ApiError> {
    let issue_id = parse_id::<TopicItemId>(&issue_id)?;
    let reply_to_comment_id = request
        .reply_to_comment_id
        .map(|value| parse_id::<CommentId>(&value))
        .transpose()?;
    let creation = IssueService::new(state.database)
        .create_comment(
            &principal,
            issue_id,
            request.kind,
            request.body,
            reply_to_comment_id,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(creation_response(creation))))
}

pub async fn list_comments(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(issue_id): Path<String>,
) -> Result<Json<Data<Vec<crate::domain::Comment>>>, ApiError> {
    let issue_id = parse_id::<TopicItemId>(&issue_id)?;
    let comments = IssueService::new(state.database)
        .list_comments(&principal, issue_id)
        .await?;
    Ok(Json(Data { data: comments }))
}

fn creation_response<T>(creation: StoredCreation<T>) -> CreationResponse<T> {
    CreationResponse {
        data: creation.value,
        dispatch: creation.dispatch_id.map(|id| DispatchResponse {
            id,
            status: "pending",
        }),
    }
}

fn parse_id<T>(value: &str) -> Result<T, ApiError>
where
    T: FromStr,
{
    value
        .parse()
        .map_err(|_| ApiError::BadRequest("identifier is invalid"))
}

const fn default_comment_kind() -> CommentKind {
    CommentKind::Discussion
}
