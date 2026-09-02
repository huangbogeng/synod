use axum::{
    Json,
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use serde::Serialize;

use crate::domain::Principal;

use super::{ApiError, AppState};

pub struct AuthenticatedPrincipal(pub Principal);

impl FromRequestParts<AppState> for AuthenticatedPrincipal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;
        let (scheme, token) = value.split_once(' ').ok_or(ApiError::Unauthorized)?;
        if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
            return Err(ApiError::Unauthorized);
        }

        let principal = state.database.authenticate(token).await.map_err(|error| {
            tracing::error!(%error, "authentication storage lookup failed");
            ApiError::Internal
        })?;
        principal.map(Self).ok_or(ApiError::Unauthorized)
    }
}

#[derive(Debug, Serialize)]
pub struct Data<T> {
    pub data: T,
}

pub async fn me(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
) -> Json<Data<Principal>> {
    Json(Data { data: principal })
}
