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

mod admin;
mod auth;
mod dispatches;
mod error;
mod issues;
mod members;
mod topics;
mod web;

use error::ApiError;

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(web::index))
        .route("/assets/app.js", get(web::javascript))
        .route("/assets/index.css", get(web::stylesheet))
        .route("/api/v1/health", get(health))
        .route("/api/v1/me", get(auth::me))
        .route("/api/v1/notifications", get(dispatches::list_notifications))
        .route(
            "/api/v1/dispatches/{dispatch_id}",
            get(dispatches::get_dispatch),
        )
        .route("/api/v1/runs/{run_id}", get(dispatches::get_run))
        .route("/api/v1/topics/{topic_id}/runs", get(dispatches::list_runs))
        .route(
            "/api/v1/context-snapshots/{snapshot_id}",
            get(dispatches::get_context_snapshot),
        )
        .route("/api/v1/topics", get(topics::list).post(topics::create))
        .route("/api/v1/topics/{topic_id}", get(topics::get))
        .route("/api/v1/issue-types", get(issues::list_types))
        .route(
            "/api/v1/providers",
            get(admin::list_providers).post(admin::create_provider),
        )
        .route(
            "/api/v1/providers/{provider_id}/models",
            get(admin::discover_models),
        )
        .route(
            "/api/v1/models",
            get(admin::list_models).post(admin::create_model),
        )
        .route(
            "/api/v1/ai-members",
            get(admin::list_ai_members).post(admin::create_ai_member),
        )
        .route(
            "/api/v1/topics/{topic_id}/members",
            get(members::list_topic_members),
        )
        .route(
            "/api/v1/topics/{topic_id}/members/{principal_id}",
            axum::routing::put(members::put_topic_member),
        )
        .route(
            "/api/v1/topics/{topic_id}/teams",
            get(members::list_teams).post(members::create_team),
        )
        .route(
            "/api/v1/teams/{team_id}/members/{principal_id}",
            axum::routing::put(members::put_team_member),
        )
        .route(
            "/api/v1/topics/{topic_id}/issues",
            get(issues::list).post(issues::create),
        )
        .route("/api/v1/issues/{issue_id}", get(issues::get))
        .route(
            "/api/v1/issues/{issue_id}/comments",
            get(issues::list_comments).post(issues::create_comment),
        )
        .fallback(web::fallback)
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
    use crate::services::TopicService;

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
    async fn embedded_web_application_and_assets_are_served_locally() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let app = router(AppState { database });

        let index = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(index.status(), StatusCode::OK);
        let index_type = index.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        assert!(index_type.starts_with("text/html"));
        let index_body = index.into_body().collect().await.unwrap().to_bytes();
        assert!(String::from_utf8_lossy(&index_body).contains("<title>Synod</title>"));

        let javascript = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(javascript.status(), StatusCode::OK);
        assert!(
            javascript.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/javascript")
        );

        let spa_route = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/topics/local-room")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(spa_route.status(), StatusCode::OK);

        let missing_api = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_api.status(), StatusCode::NOT_FOUND);
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

    #[tokio::test]
    async fn issue_comment_and_mention_flow_is_persisted() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let bootstrap = database
            .bootstrap_human("alice", "Alice", "test")
            .await
            .unwrap();
        let topic = TopicService::new(database.clone())
            .create(
                &bootstrap.principal,
                "factor-lab".to_owned(),
                "Factor Lab".to_owned(),
                String::new(),
            )
            .await
            .unwrap();
        let bearer = format!("Bearer {}", bootstrap.token);
        let app = router(AppState {
            database: database.clone(),
        });

        let types = send(&app, "GET", "/api/v1/issue-types", &bearer, None).await;
        assert_eq!(types.status(), StatusCode::OK);
        assert_eq!(json_body(types).await["data"].as_array().unwrap().len(), 7);

        let issue = send(
            &app,
            "POST",
            &format!("/api/v1/topics/{}/issues", topic.id),
            &bearer,
            Some(serde_json::json!({
                "issue_type": "research",
                "title": "Test revision signals",
                "body": "@Architect inspect this. `@ignored` Then @architect again."
            })),
        )
        .await;
        assert_eq!(issue.status(), StatusCode::CREATED);
        let issue = json_body(issue).await;
        let issue_id = issue["data"]["id"].as_str().unwrap();
        assert_eq!(issue["data"]["number"], 1);
        assert_eq!(issue["dispatch"]["status"], "pending");

        let handles: Vec<String> = sqlx::query_scalar(
            "SELECT mention.handle FROM dispatch_mentions AS mention
             JOIN dispatches AS dispatch ON dispatch.id = mention.dispatch_id
             WHERE dispatch.source_id = ? ORDER BY mention.mention_order",
        )
        .bind(issue_id)
        .fetch_all(database.pool())
        .await
        .unwrap();
        assert_eq!(handles, ["architect"]);

        let child = send(
            &app,
            "POST",
            &format!("/api/v1/topics/{}/issues", topic.id),
            &bearer,
            Some(serde_json::json!({
                "issue_type": "experiment",
                "title": "Measure the signal",
                "body": "No mention",
                "parent_issue_id": issue_id
            })),
        )
        .await;
        assert_eq!(child.status(), StatusCode::CREATED);
        let child = json_body(child).await;
        assert_eq!(child["data"]["number"], 2);
        assert_eq!(child["data"]["parent_issue_id"], issue_id);
        assert!(child["dispatch"].is_null());

        let comment = send(
            &app,
            "POST",
            &format!("/api/v1/issues/{issue_id}/comments"),
            &bearer,
            Some(serde_json::json!({
                "kind": "direction",
                "body": "@security-team focus on publication timing."
            })),
        )
        .await;
        assert_eq!(comment.status(), StatusCode::CREATED);
        let comment = json_body(comment).await;
        assert_eq!(comment["data"]["kind"], "direction");
        assert_eq!(comment["dispatch"]["status"], "pending");

        let comments = send(
            &app,
            "GET",
            &format!("/api/v1/issues/{issue_id}/comments"),
            &bearer,
            None,
        )
        .await;
        assert_eq!(comments.status(), StatusCode::OK);
        assert_eq!(
            json_body(comments).await["data"].as_array().unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn provider_model_ai_member_and_team_flow_is_consistent() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let bootstrap = database
            .bootstrap_human("alice", "Alice", "test")
            .await
            .unwrap();
        let topic = TopicService::new(database.clone())
            .create(
                &bootstrap.principal,
                "factor-lab".to_owned(),
                "Factor Lab".to_owned(),
                String::new(),
            )
            .await
            .unwrap();
        let bearer = format!("Bearer {}", bootstrap.token);
        let app = router(AppState {
            database: database.clone(),
        });

        let malformed_discovery = send(
            &app,
            "GET",
            "/api/v1/providers/not-an-id/models",
            &bearer,
            None,
        )
        .await;
        assert_eq!(malformed_discovery.status(), StatusCode::BAD_REQUEST);
        let missing_discovery = send(
            &app,
            "GET",
            &format!(
                "/api/v1/providers/{}/models",
                crate::domain::ProviderId::new()
            ),
            &bearer,
            None,
        )
        .await;
        assert_eq!(missing_discovery.status(), StatusCode::NOT_FOUND);

        let raw_secret = send(
            &app,
            "POST",
            "/api/v1/providers",
            &bearer,
            Some(serde_json::json!({
                "name": "Unsafe",
                "adapter": "openai_compatible",
                "base_url": "https://example.com",
                "credential_ref": "raw-secret"
            })),
        )
        .await;
        assert_eq!(raw_secret.status(), StatusCode::BAD_REQUEST);

        let ambiguous_secret = send(
            &app,
            "POST",
            "/api/v1/providers",
            &bearer,
            Some(serde_json::json!({
                "name": "Ambiguous",
                "adapter": "openai_compatible",
                "base_url": "https://api.deepseek.com",
                "credential_ref": "env://DEEPSEEK_API_KEY",
                "api_key": "must-not-be-stored"
            })),
        )
        .await;
        assert_eq!(ambiguous_secret.status(), StatusCode::BAD_REQUEST);
        let leaked_ambiguous: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM provider_secrets WHERE secret = ?")
                .bind("must-not-be-stored")
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(leaked_ambiguous, 0);

        let local_secret = send(
            &app,
            "POST",
            "/api/v1/providers",
            &bearer,
            Some(serde_json::json!({
                "name": "MiniMax Local",
                "adapter": "openai_compatible",
                "base_url": "https://api.minimaxi.com/v1",
                "api_key": "local-test-secret"
            })),
        )
        .await;
        assert_eq!(local_secret.status(), StatusCode::CREATED);
        let local_secret = json_body(local_secret).await;
        assert_eq!(local_secret["data"]["credential_configured"], true);
        assert!(local_secret["data"].get("credential_ref").is_none());
        assert!(local_secret["data"].get("api_key").is_none());
        let local_provider_id = local_secret["data"]["id"].as_str().unwrap();
        let stored_secret: String =
            sqlx::query_scalar("SELECT secret FROM provider_secrets WHERE provider_id = ?")
                .bind(local_provider_id)
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(stored_secret, "local-test-secret");
        assert_eq!(
            database
                .resolve_provider_secret(&format!("secret://{local_provider_id}"))
                .await
                .unwrap()
                .as_deref(),
            Some("local-test-secret")
        );

        let provider = send(
            &app,
            "POST",
            "/api/v1/providers",
            &bearer,
            Some(serde_json::json!({
                "name": "DeepSeek",
                "adapter": "openai_compatible",
                "base_url": "https://api.deepseek.com",
                "credential_ref": "env://DEEPSEEK_API_KEY"
            })),
        )
        .await;
        assert_eq!(provider.status(), StatusCode::CREATED);
        let provider = json_body(provider).await;
        assert_eq!(provider["data"]["credential_configured"], true);
        assert!(provider["data"].get("credential_ref").is_none());
        let provider_id = provider["data"]["id"].as_str().unwrap();

        let model = send(
            &app,
            "POST",
            "/api/v1/models",
            &bearer,
            Some(serde_json::json!({
                "provider_id": provider_id,
                "model_name": "configured-deepseek-model",
                "display_name": "Reasoning Model",
                "capabilities": {"streaming": false, "tool_calling": false}
            })),
        )
        .await;
        assert_eq!(model.status(), StatusCode::CREATED);
        let model = json_body(model).await;
        let model_id = model["data"]["id"].as_str().unwrap();

        let ai_member = send(
            &app,
            "POST",
            "/api/v1/ai-members",
            &bearer,
            Some(serde_json::json!({
                "handle": "architect",
                "display_name": "Architect",
                "identity_prompt": "Review system boundaries.",
                "default_model_id": model_id
            })),
        )
        .await;
        assert_eq!(ai_member.status(), StatusCode::CREATED);
        let ai_member = json_body(ai_member).await;
        let ai_id = ai_member["data"]["id"].as_str().unwrap();
        assert_eq!(ai_member["data"]["kind"], "ai");

        let reviewer = send(
            &app,
            "POST",
            "/api/v1/ai-members",
            &bearer,
            Some(serde_json::json!({
                "handle": "reviewer",
                "display_name": "Reviewer",
                "identity_prompt": "Challenge the proposed implementation.",
                "provider_id": provider_id,
                "model_name": "configured-deepseek-model"
            })),
        )
        .await;
        assert_eq!(reviewer.status(), StatusCode::CREATED);
        let configured_model_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM models WHERE provider_id = ? AND model_name = ?",
        )
        .bind(provider_id)
        .bind("configured-deepseek-model")
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(configured_model_count, 1);

        let duplicate = send(
            &app,
            "POST",
            "/api/v1/ai-members",
            &bearer,
            Some(serde_json::json!({
                "handle": "architect",
                "display_name": "Duplicate Architect",
                "identity_prompt": "This member must not be created.",
                "provider_id": provider_id,
                "model_name": "must-rollback-model"
            })),
        )
        .await;
        assert_eq!(duplicate.status(), StatusCode::CONFLICT);
        let orphan_model_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM models WHERE model_name = ?")
                .bind("must-rollback-model")
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(orphan_model_count, 0);

        let membership = send(
            &app,
            "PUT",
            &format!("/api/v1/topics/{}/members/{ai_id}", topic.id),
            &bearer,
            Some(serde_json::json!({"role": "contribute"})),
        )
        .await;
        assert_eq!(membership.status(), StatusCode::OK);
        assert_eq!(json_body(membership).await["data"]["role"], "contribute");

        let team = send(
            &app,
            "POST",
            &format!("/api/v1/topics/{}/teams", topic.id),
            &bearer,
            Some(serde_json::json!({
                "handle": "design-team",
                "display_name": "Design Team"
            })),
        )
        .await;
        assert_eq!(team.status(), StatusCode::CREATED);
        let team = json_body(team).await;
        let team_id = team["data"]["id"].as_str().unwrap();

        let team = send(
            &app,
            "PUT",
            &format!("/api/v1/teams/{team_id}/members/{ai_id}"),
            &bearer,
            None,
        )
        .await;
        assert_eq!(team.status(), StatusCode::OK);
        let team = json_body(team).await;
        assert_eq!(team["data"]["members"].as_array().unwrap().len(), 1);

        let teams = send(
            &app,
            "GET",
            &format!("/api/v1/topics/{}/teams", topic.id),
            &bearer,
            None,
        )
        .await;
        assert_eq!(teams.status(), StatusCode::OK);
        assert_eq!(json_body(teams).await["data"].as_array().unwrap().len(), 1);

        let issue = send(
            &app,
            "POST",
            &format!("/api/v1/topics/{}/issues", topic.id),
            &bearer,
            Some(serde_json::json!({
                "issue_type": "code_audit",
                "title": "Review boundaries",
                "body": "@design-team review this design."
            })),
        )
        .await;
        assert_eq!(issue.status(), StatusCode::CREATED);
        let issue = json_body(issue).await;
        let dispatch_id = issue["dispatch"]["id"].as_str().unwrap();
        assert!(crate::workers::process_once(&database).await.unwrap());

        let dispatch = send(
            &app,
            "GET",
            &format!("/api/v1/dispatches/{dispatch_id}"),
            &bearer,
            None,
        )
        .await;
        assert_eq!(dispatch.status(), StatusCode::OK);
        let dispatch = json_body(dispatch).await;
        assert_eq!(dispatch["data"]["status"], "dispatched");
        assert_eq!(dispatch["data"]["targets"][0]["handle"], "architect");
        let run_id = dispatch["data"]["targets"][0]["run_id"].as_str().unwrap();

        let run = send(
            &app,
            "GET",
            &format!("/api/v1/runs/{run_id}"),
            &bearer,
            None,
        )
        .await;
        assert_eq!(run.status(), StatusCode::OK);
        let run = json_body(run).await;
        assert_eq!(run["data"]["status"], "queued");
        assert_eq!(run["data"]["ai_member_id"], ai_id);

        let runs = send(
            &app,
            "GET",
            &format!("/api/v1/topics/{}/runs", topic.id),
            &bearer,
            None,
        )
        .await;
        assert_eq!(runs.status(), StatusCode::OK);
        let runs = json_body(runs).await;
        assert_eq!(runs["data"].as_array().unwrap().len(), 1);
        assert_eq!(runs["data"][0]["id"], run_id);
    }

    async fn send(
        app: &Router,
        method: &str,
        uri: &str,
        bearer: &str,
        body: Option<serde_json::Value>,
    ) -> Response {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, bearer);
        let body = if let Some(body) = body {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        } else {
            Body::empty()
        };
        app.clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap()
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }
}
