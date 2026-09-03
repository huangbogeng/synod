use std::str::FromStr;

use sqlx::Row;

use crate::domain::{AiMember, ModelId, Principal, PrincipalId, PrincipalKind};

use super::{Database, StoreError, members};

impl Database {
    pub(crate) async fn local_bootstrap_principal(&self) -> Result<Principal, StoreError> {
        let row = sqlx::query(
            "SELECT principal.id, principal.kind, principal.handle, principal.display_name
             FROM server_state AS state
             JOIN principals AS principal ON principal.id = state.bootstrap_principal_id
             WHERE state.singleton_id = 1 AND principal.active = 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::PermissionDenied)?;
        let kind: String = row.try_get("kind")?;
        if kind != "human" {
            return Err(StoreError::PermissionDenied);
        }
        Ok(Principal {
            id: parse_id(&row, "id", "bootstrap principal id")?,
            kind: PrincipalKind::Human,
            handle: row.try_get("handle")?,
            display_name: row.try_get("display_name")?,
        })
    }

    pub(crate) async fn clear_all_topics_local(
        &self,
        actor_id: PrincipalId,
    ) -> Result<u64, StoreError> {
        self.require_server_admin(actor_id).await?;
        let mut transaction = self.pool.begin().await?;

        // Cross-links use RESTRICT to protect ordinary mutations. A deliberate
        // full reset removes those dependants first, then lets Topic cascades
        // handle the remaining ownership tree.
        for statement in [
            "DELETE FROM dispatch_target_sources",
            "DELETE FROM dispatch_targets",
            "DELETE FROM provider_attempts",
            "DELETE FROM context_snapshots",
            "DELETE FROM conversation_items",
            "DELETE FROM notifications",
            "UPDATE runs SET retry_of_run_id = NULL",
            "DELETE FROM runs",
            "DELETE FROM conversations",
            "DELETE FROM dispatch_mentions",
            "DELETE FROM dispatches",
            "DELETE FROM comment_revisions",
            "UPDATE comments SET reply_to_comment_id = NULL",
            "DELETE FROM comments",
            "UPDATE issues SET parent_issue_item_id = NULL",
            "DELETE FROM issues",
            "DELETE FROM proposals",
            "DELETE FROM topic_items",
            "DELETE FROM team_members",
            "DELETE FROM teams",
            "DELETE FROM topic_memberships",
            "DELETE FROM activity_events",
            "DELETE FROM jobs",
        ] {
            sqlx::query(statement).execute(&mut *transaction).await?;
        }
        let deleted = sqlx::query("DELETE FROM topics")
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        transaction.commit().await?;
        Ok(deleted)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn configure_ai_member_local(
        &self,
        actor_id: PrincipalId,
        handle: &str,
        display_name: &str,
        identity_prompt: &str,
        provider_name: &str,
        model_name: &str,
        execution_defaults: &serde_json::Value,
    ) -> Result<AiMember, StoreError> {
        self.require_server_admin(actor_id).await?;
        let mut transaction = self.pool.begin().await?;
        let model_id: String = sqlx::query_scalar(
            "SELECT model.id
             FROM models AS model
             JOIN providers AS provider ON provider.id = model.provider_id
             WHERE provider.name = ? COLLATE NOCASE AND provider.enabled = 1
               AND model.model_name = ? AND model.enabled = 1",
        )
        .bind(provider_name)
        .bind(model_name)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(StoreError::InvalidReference(
            "enabled Provider or Model was not found",
        ))?;
        let model_id = ModelId::from_str(&model_id)
            .map_err(|_| StoreError::CorruptData("model identifier is invalid"))?;

        let existing = sqlx::query(
            "SELECT principal.id, principal.kind, profile.identity_prompt_version,
                    prompt.prompt
             FROM principals AS principal
             LEFT JOIN ai_profiles AS profile ON profile.principal_id = principal.id
             LEFT JOIN ai_prompt_versions AS prompt
               ON prompt.ai_principal_id = profile.principal_id
              AND prompt.version = profile.identity_prompt_version
             WHERE principal.handle = ? COLLATE NOCASE",
        )
        .bind(handle)
        .fetch_optional(&mut *transaction)
        .await?;

        let principal_id = if let Some(existing) = existing {
            let kind: String = existing.try_get("kind")?;
            if kind != "ai" {
                return Err(StoreError::Conflict);
            }
            let principal_id: PrincipalId = parse_id(&existing, "id", "AI principal id")?;
            let current_version: i64 = existing.try_get("identity_prompt_version")?;
            let current_prompt: String = existing.try_get("prompt")?;
            let next_version = if current_prompt == identity_prompt {
                current_version
            } else {
                let next_version = current_version + 1;
                sqlx::query(
                    "INSERT INTO ai_prompt_versions(
                        ai_principal_id, version, prompt, created_by_principal_id, created_at
                     ) VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                )
                .bind(principal_id.to_string())
                .bind(next_version)
                .bind(identity_prompt)
                .bind(actor_id.to_string())
                .execute(&mut *transaction)
                .await?;
                next_version
            };
            sqlx::query("UPDATE principals SET display_name = ?, active = 1 WHERE id = ?")
                .bind(display_name)
                .bind(principal_id.to_string())
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "UPDATE ai_profiles
                 SET identity_prompt_version = ?, default_model_id = ?,
                     execution_defaults_json = ?
                 WHERE principal_id = ?",
            )
            .bind(next_version)
            .bind(model_id.to_string())
            .bind(execution_defaults.to_string())
            .bind(principal_id.to_string())
            .execute(&mut *transaction)
            .await?;
            principal_id
        } else {
            members::insert_ai_member_in_transaction(
                &mut transaction,
                actor_id,
                handle,
                display_name,
                identity_prompt,
                model_id,
                execution_defaults,
            )
            .await?
            .principal
            .id
        };

        let row = sqlx::query(
            "SELECT principal.id, principal.kind, principal.handle, principal.display_name,
                    prompt.prompt AS identity_prompt,
                    profile.identity_prompt_version, profile.default_model_id,
                    profile.execution_defaults_json
             FROM principals AS principal
             JOIN ai_profiles AS profile ON profile.principal_id = principal.id
             JOIN ai_prompt_versions AS prompt
               ON prompt.ai_principal_id = profile.principal_id
              AND prompt.version = profile.identity_prompt_version
             WHERE principal.id = ?",
        )
        .bind(principal_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;
        let member = members::ai_member_from_row(&row)?;
        transaction.commit().await?;
        Ok(member)
    }
}

fn parse_id<T>(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
    label: &'static str,
) -> Result<T, StoreError>
where
    T: FromStr,
{
    let raw: String = row.try_get(column)?;
    raw.parse().map_err(|_| StoreError::CorruptData(label))
}
