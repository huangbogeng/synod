use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    domain::{DispatchId, RunId},
    services::DispatchService,
};

use super::{
    AppState,
    auth::{AuthenticatedPrincipal, Data},
    error::ApiError,
};

pub async fn get_dispatch(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(dispatch_id): Path<String>,
) -> Result<Json<Data<crate::domain::Dispatch>>, ApiError> {
    let dispatch_id = parse_id::<DispatchId>(&dispatch_id)?;
    let dispatch = DispatchService::new(state.database)
        .get_dispatch(&principal, dispatch_id)
        .await?;
    Ok(Json(Data { data: dispatch }))
}

pub async fn get_run(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    Path(run_id): Path<String>,
) -> Result<Json<Data<crate::domain::Run>>, ApiError> {
    let run_id = parse_id::<RunId>(&run_id)?;
    let run = DispatchService::new(state.database)
        .get_run(&principal, run_id)
        .await?;
    Ok(Json(Data { data: run }))
}

pub async fn list_notifications(
    State(state): State<AppState>,
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
) -> Result<Json<Data<Vec<crate::domain::Notification>>>, ApiError> {
    let notifications = DispatchService::new(state.database)
        .list_notifications(&principal)
        .await?;
    Ok(Json(Data {
        data: notifications,
    }))
}

fn parse_id<T>(value: &str) -> Result<T, ApiError>
where
    T: FromStr,
{
    value
        .parse()
        .map_err(|_| ApiError::BadRequest("identifier is invalid"))
}
