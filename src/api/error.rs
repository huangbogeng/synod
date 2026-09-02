use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::services::ServiceError;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(&'static str),
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "valid bearer authentication is required",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "operation is not permitted",
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "resource was not found"),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "conflict",
                "resource already exists or has changed",
            ),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "an internal error occurred",
            ),
        };

        (
            status,
            Json(ErrorEnvelope {
                error: ErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

impl From<ServiceError> for ApiError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::Validation(error) => {
                tracing::debug!(%error, "request validation failed");
                Self::BadRequest("request content is invalid")
            }
            ServiceError::Forbidden => Self::Forbidden,
            ServiceError::Conflict => Self::Conflict,
            ServiceError::NotFound => Self::NotFound,
            ServiceError::Storage(error) => {
                tracing::error!(%error, "storage operation failed");
                Self::Internal
            }
            ServiceError::CorruptData => {
                tracing::error!("stored data failed validation");
                Self::Internal
            }
        }
    }
}
