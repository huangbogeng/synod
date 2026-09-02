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

mod auth;
mod error;
mod topics;

use error::ApiError;

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/me", get(auth::me))
        .route("/api/v1/topics", get(topics::list).post(topics::create))
        .route("/api/v1/topics/{topic_id}", get(topics::get))
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

    use axum::{
        body::Body,
        http::{Request, header},
    };
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

    #[tokio::test]
    async fn authenticated_human_can_create_and_read_a_topic() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let bootstrap = database
            .bootstrap_human("alice", "Alice", "test")
            .await
            .unwrap();
        let bearer = format!("Bearer {}", bootstrap.token);
        let inspection_database = database.clone();
        let app = router(AppState { database });

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header(header::AUTHORIZATION, &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), StatusCode::OK);
        assert_eq!(json_body(me).await["data"]["handle"], "alice");

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/topics")
                    .header(header::AUTHORIZATION, &bearer)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "key": "factor-lab",
                            "title": "Factor Lab",
                            "description": "Research factors"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::CREATED);
        let created = json_body(create).await;
        let topic_id = created["data"]["id"].as_str().unwrap();
        assert_eq!(created["data"]["key"], "factor-lab");

        let audit_events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM activity_events WHERE event_type = ?")
                .bind("topic.created")
                .fetch_one(inspection_database.pool())
                .await
                .unwrap();
        assert_eq!(audit_events, 1);

        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/topics")
                    .header(header::AUTHORIZATION, &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        assert_eq!(json_body(list).await["data"].as_array().unwrap().len(), 1);

        let get = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/topics/{topic_id}"))
                    .header(header::AUTHORIZATION, &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::OK);
        assert_eq!(json_body(get).await["data"]["title"], "Factor Lab");
    }

    #[tokio::test]
    async fn topic_routes_require_authentication() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let response = router(AppState { database })
            .oneshot(
                Request::builder()
                    .uri("/api/v1/topics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(json_body(response).await["error"]["code"], "unauthorized");
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }
}
