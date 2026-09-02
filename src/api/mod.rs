use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tower_http::trace::TraceLayer;

use crate::persistence::Database;

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
    version: &'static str,
}

async fn health(State(state): State<AppState>) -> Response {
    let (status, database, code) = match state.database.healthcheck().await {
        Ok(()) => ("ok", "ok", StatusCode::OK),
        Err(error) => {
            tracing::error!(%error, "database health check failed");
            ("degraded", "unavailable", StatusCode::SERVICE_UNAVAILABLE)
        }
    };

    (
        code,
        Json(HealthResponse {
            status,
            database,
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_reports_a_ready_database() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let response = router(AppState { database })
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["database"], "ok");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }
}
