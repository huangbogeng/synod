use crate::{persistence::Database, providers::ModelGateway};

use super::ServiceError;

#[derive(Debug, Clone)]
pub struct ExecutionService {
    database: Database,
}

impl ExecutionService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn execute_once<G>(&self, gateway: &G) -> Result<bool, ServiceError>
    where
        G: ModelGateway,
    {
        let Some(claimed) = self.database.claim_next_run().await? else {
            return Ok(false);
        };
        match gateway
            .complete(claimed.route.clone(), claimed.request.clone())
            .await
        {
            Ok(response)
                if !response.text.trim().is_empty() && response.text.chars().count() <= 500_000 =>
            {
                self.database
                    .complete_claimed_run(&claimed, &response)
                    .await?;
            }
            Ok(_) => {
                self.database
                    .fail_claimed_run(&claimed, "provider returned an empty or oversized response")
                    .await?;
            }
            Err(error) => {
                tracing::warn!(run_id = %claimed.request.run_id, %error, "provider execution failed");
                self.database
                    .fail_claimed_run(&claimed, &error.to_string())
                    .await?;
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::{future, path::Path, sync::Mutex};

    use crate::{
        domain::{
            MembershipRole, ModelRequest, ModelResponse, Principal, ProviderAdapter, RunConclusion,
            RunStatus, TopicItemId,
        },
        persistence::Database,
        providers::{ModelGateway, ProviderError, ProviderRoute},
        services::{AdminService, IssueService, MembershipService, TopicService},
    };

    use super::*;

    struct RecordingGateway {
        seen: Mutex<Option<(ProviderRoute, ModelRequest)>>,
    }

    impl RecordingGateway {
        fn new() -> Self {
            Self {
                seen: Mutex::new(None),
            }
        }
    }

    impl ModelGateway for RecordingGateway {
        fn complete(
            &self,
            route: ProviderRoute,
            request: ModelRequest,
        ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send {
            *self.seen.lock().unwrap() = Some((route, request));
            future::ready(Ok(ModelResponse {
                text: "The boundary is sound, but document the retry policy.".to_owned(),
                usage: serde_json::json!({"input_tokens": 120, "output_tokens": 14}),
                provider_request_id: Some("request-1".to_owned()),
            }))
        }
    }

    struct FailingGateway;

    impl ModelGateway for FailingGateway {
        fn complete(
            &self,
            _route: ProviderRoute,
            _request: ModelRequest,
        ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send {
            future::ready(Err(ProviderError::Request(
                "temporary upstream error".to_owned(),
            )))
        }
    }

    #[tokio::test]
    async fn execution_freezes_context_and_publishes_an_ai_comment() {
        let (database, alice, issue_id, run_id) = queued_run().await;
        let gateway = RecordingGateway::new();
        let execution = ExecutionService::new(database.clone());

        assert!(execution.execute_once(&gateway).await.unwrap());
        assert!(!execution.execute_once(&gateway).await.unwrap());

        {
            let seen = gateway.seen.lock().unwrap();
            let (route, request) = seen.as_ref().unwrap();
            assert_eq!(route.adapter, ProviderAdapter::OpenaiCompatible);
            assert_eq!(route.credential_ref, "env://TEST_API_KEY");
            assert_eq!(request.context.issue.id, issue_id);
            assert_eq!(request.context.trigger.source_type, "issue");
            assert!(request.system_prompt.contains("Review architecture."));
        }

        let run = database.get_run_for(alice.id, run_id).await.unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.conclusion, Some(RunConclusion::Success));
        assert!(run.context_snapshot_id.is_some());
        let snapshot = database
            .get_context_snapshot_for(alice.id, run.context_snapshot_id.unwrap())
            .await
            .unwrap();
        assert_eq!(snapshot.run_id, run_id);
        assert_eq!(
            snapshot.input.topic.description,
            "Keep the system lightweight."
        );
        assert!(snapshot.manifest.omissions.is_empty());

        let comments = IssueService::new(database.clone())
            .list_comments(&alice, issue_id)
            .await
            .unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(
            comments[0].body,
            "The boundary is sound, but document the retry policy."
        );
        assert_ne!(comments[0].author_id, alice.id);

        let attempt: (String, String, String) = sqlx::query_as(
            "SELECT status, conclusion, usage_json FROM provider_attempts WHERE run_id = ?",
        )
        .bind(run_id.to_string())
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(attempt.0, "completed");
        assert_eq!(attempt.1, "success");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&attempt.2).unwrap()["input_tokens"],
            120
        );
        let item_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM conversation_items WHERE run_id = ?")
                .bind(run_id.to_string())
                .fetch_one(database.pool())
                .await
                .unwrap();
        let dispatch_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dispatches")
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert_eq!(item_count, 2);
        assert_eq!(dispatch_count, 1, "AI output must not recursively dispatch");
    }

    #[tokio::test]
    async fn provider_failure_is_terminal_without_a_fake_comment() {
        let (database, alice, issue_id, run_id) = queued_run().await;
        assert!(
            ExecutionService::new(database.clone())
                .execute_once(&FailingGateway)
                .await
                .unwrap()
        );

        let run = database.get_run_for(alice.id, run_id).await.unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.conclusion, Some(RunConclusion::Failure));
        assert!(
            IssueService::new(database.clone())
                .list_comments(&alice, issue_id)
                .await
                .unwrap()
                .is_empty()
        );
        let job_outcome: String = sqlx::query_scalar(
            "SELECT outcome FROM jobs WHERE json_extract(payload, '$.run_id') = ?",
        )
        .bind(run_id.to_string())
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(job_outcome, "failure");
    }

    #[tokio::test]
    async fn expired_lease_is_fenced_and_reuses_the_frozen_context() {
        let (database, _alice, _issue_id, run_id) = queued_run().await;
        let first = database.claim_next_run().await.unwrap().unwrap();
        sqlx::query("UPDATE jobs SET lease_expires_at = '2000-01-01T00:00:00.000Z' WHERE id = ?")
            .bind(first.job_id.to_string())
            .execute(database.pool())
            .await
            .unwrap();

        let second = database.claim_next_run().await.unwrap().unwrap();
        assert_ne!(first.lease_token, second.lease_token);
        assert_eq!(
            first.request.context_snapshot_id,
            second.request.context_snapshot_id
        );
        let response = ModelResponse {
            text: "Recovered response.".to_owned(),
            usage: serde_json::json!({}),
            provider_request_id: None,
        };
        assert!(matches!(
            database.complete_claimed_run(&first, &response).await,
            Err(crate::persistence::StoreError::Conflict)
        ));
        database
            .complete_claimed_run(&second, &response)
            .await
            .unwrap();

        let attempts: Vec<(String, String)> = sqlx::query_as(
            "SELECT status, conclusion FROM provider_attempts
             WHERE run_id = ? ORDER BY sequence",
        )
        .bind(run_id.to_string())
        .fetch_all(database.pool())
        .await
        .unwrap();
        assert_eq!(
            attempts,
            vec![
                ("completed".to_owned(), "timed_out".to_owned()),
                ("completed".to_owned(), "success".to_owned()),
            ]
        );
    }

    async fn queued_run() -> (Database, Principal, TopicItemId, crate::domain::RunId) {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let alice = database
            .bootstrap_human("alice", "Alice", "test")
            .await
            .unwrap()
            .principal;
        let topic = TopicService::new(database.clone())
            .create(
                &alice,
                "synod".to_owned(),
                "Synod".to_owned(),
                "Keep the system lightweight.".to_owned(),
            )
            .await
            .unwrap();
        let admin = AdminService::new(database.clone());
        let provider = admin
            .create_provider(
                &alice,
                "Test".to_owned(),
                ProviderAdapter::OpenaiCompatible,
                "https://example.test/v1".to_owned(),
                "env://TEST_API_KEY".to_owned(),
            )
            .await
            .unwrap();
        let model = admin
            .create_model(
                &alice,
                provider.id,
                "test-model".to_owned(),
                "Test Model".to_owned(),
                serde_json::json!({}),
                serde_json::json!({"context_tokens": 32000}),
                serde_json::json!({"temperature": 0}),
            )
            .await
            .unwrap();
        let architect = admin
            .create_ai_member(
                &alice,
                "architect".to_owned(),
                "Architect".to_owned(),
                "Review architecture.".to_owned(),
                model.id,
            )
            .await
            .unwrap();
        MembershipService::new(database.clone())
            .put_topic_member(
                &alice,
                topic.id,
                architect.principal.id,
                MembershipRole::Contribute,
            )
            .await
            .unwrap();
        let issue = IssueService::new(database.clone())
            .create_issue(
                &alice,
                topic.id,
                "code_audit".to_owned(),
                "Audit execution".to_owned(),
                "@architect inspect the execution boundary.".to_owned(),
                None,
            )
            .await
            .unwrap();
        let dispatch_id = issue.dispatch_id.unwrap();
        assert!(database.resolve_next_dispatch().await.unwrap());
        let dispatch = database
            .get_dispatch_for(alice.id, dispatch_id)
            .await
            .unwrap();
        let run_id = dispatch.targets[0].run_id.unwrap();
        (database, alice, issue.value.id, run_id)
    }
}
