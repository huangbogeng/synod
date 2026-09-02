use std::str::FromStr;

use sha2::{Digest, Sha256};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::domain::{
    CreateTopic, MembershipRole, Principal, PrincipalId, PrincipalKind, Topic, TopicId,
};

use super::Database;

pub struct BootstrapOutput {
    pub principal: Principal,
    pub token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("server already has a bootstrap principal")]
    AlreadyBootstrapped,
    #[error("resource already exists")]
    Conflict,
    #[error("resource was not found")]
    NotFound,
    #[error("operation is not permitted")]
    PermissionDenied,
    #[error("invalid reference: {0}")]
    InvalidReference(&'static str),
    #[error("stored data is invalid: {0}")]
    CorruptData(&'static str),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl Database {
    pub async fn bootstrap_human(
        &self,
        handle: &str,
        display_name: &str,
        token_label: &str,
    ) -> Result<BootstrapOutput, StoreError> {
        let principal_id = PrincipalId::new();
        let token_id = Uuid::now_v7();
        let token = generate_token();
        let token_hash = hash_token(&token);
        let mut transaction = self.pool.begin().await?;

        let existing: Option<String> = sqlx::query_scalar(
            "SELECT bootstrap_principal_id FROM server_state WHERE singleton_id = 1",
        )
        .fetch_one(&mut *transaction)
        .await?;
        if existing.is_some() {
            return Err(StoreError::AlreadyBootstrapped);
        }

        sqlx::query(
            "INSERT INTO principals(id, kind, handle, display_name, created_at)
             VALUES (?, 'human', ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(principal_id.to_string())
        .bind(handle)
        .bind(display_name)
        .execute(&mut *transaction)
        .await
        .map_err(map_constraint)?;

        let claimed = sqlx::query(
            "UPDATE server_state SET bootstrap_principal_id = ?
             WHERE singleton_id = 1 AND bootstrap_principal_id IS NULL",
        )
        .bind(principal_id.to_string())
        .execute(&mut *transaction)
        .await?;

        if claimed.rows_affected() != 1 {
            return Err(StoreError::AlreadyBootstrapped);
        }

        sqlx::query(
            "INSERT INTO principal_tokens(
                id, principal_id, label, token_hash, created_at
             ) VALUES (?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(token_id.to_string())
        .bind(principal_id.to_string())
        .bind(token_label)
        .bind(token_hash.as_slice())
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(BootstrapOutput {
            principal: Principal {
                id: principal_id,
                kind: PrincipalKind::Human,
                handle: handle.to_owned(),
                display_name: display_name.to_owned(),
            },
            token,
        })
    }

    pub async fn authenticate(&self, token: &str) -> Result<Option<Principal>, StoreError> {
        let token_hash = hash_token(token);
        let row = sqlx::query(
            "SELECT p.id, p.kind, p.handle, p.display_name
             FROM principal_tokens AS token
             JOIN principals AS p ON p.id = token.principal_id
             WHERE token.token_hash = ?
               AND token.revoked_at IS NULL
               AND (token.expires_at IS NULL OR token.expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
               AND p.active = 1",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await?;

        row.map(principal_from_row).transpose()
    }

    pub(crate) async fn insert_topic(
        &self,
        actor: &Principal,
        input: &CreateTopic,
    ) -> Result<Topic, StoreError> {
        let topic_id = TopicId::new();
        let event_id = Uuid::now_v7();
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO topics(
                id, topic_key, title, description, created_at, updated_at
             ) VALUES (?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(topic_id.to_string())
        .bind(&input.key)
        .bind(&input.title)
        .bind(&input.description)
        .execute(&mut *transaction)
        .await
        .map_err(map_constraint)?;

        insert_membership(&mut transaction, topic_id, actor.id, MembershipRole::Write).await?;

        sqlx::query(
            "INSERT INTO activity_events(
                id, topic_id, sequence, event_type, actor_principal_id,
                subject_type, subject_id, created_at
             ) VALUES (?, ?, 1, 'topic.created', ?, 'topic', ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(event_id.to_string())
        .bind(topic_id.to_string())
        .bind(actor.id.to_string())
        .bind(topic_id.to_string())
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(Topic {
            id: topic_id,
            key: input.key.clone(),
            title: input.title.clone(),
            description: input.description.clone(),
            revision: 1,
        })
    }

    pub(crate) async fn list_topics_for(
        &self,
        principal_id: PrincipalId,
    ) -> Result<Vec<Topic>, StoreError> {
        let rows = sqlx::query(
            "SELECT topic.id, topic.topic_key, topic.title, topic.description, topic.revision
             FROM topics AS topic
             JOIN topic_memberships AS membership ON membership.topic_id = topic.id
             WHERE membership.principal_id = ?
             ORDER BY topic.updated_at DESC, topic.id",
        )
        .bind(principal_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(topic_from_row).collect()
    }

    pub(crate) async fn get_topic_for(
        &self,
        principal_id: PrincipalId,
        topic_id: TopicId,
    ) -> Result<Topic, StoreError> {
        let row = sqlx::query(
            "SELECT topic.id, topic.topic_key, topic.title, topic.description, topic.revision
             FROM topics AS topic
             JOIN topic_memberships AS membership ON membership.topic_id = topic.id
             WHERE membership.principal_id = ? AND topic.id = ?",
        )
        .bind(principal_id.to_string())
        .bind(topic_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;

        topic_from_row(&row)
    }
}

async fn insert_membership(
    transaction: &mut Transaction<'_, Sqlite>,
    topic_id: TopicId,
    principal_id: PrincipalId,
    role: MembershipRole,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO topic_memberships(topic_id, principal_id, role, created_at)
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(topic_id.to_string())
    .bind(principal_id.to_string())
    .bind(role.as_str())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn principal_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Principal, StoreError> {
    let id: String = row.try_get("id")?;
    let kind: String = row.try_get("kind")?;
    Ok(Principal {
        id: PrincipalId::from_str(&id).map_err(|_| StoreError::CorruptData("principal id"))?,
        kind: match kind.as_str() {
            "human" => PrincipalKind::Human,
            "ai" => PrincipalKind::Ai,
            "caller" => PrincipalKind::Caller,
            "system" => PrincipalKind::System,
            _ => return Err(StoreError::CorruptData("principal kind")),
        },
        handle: row.try_get("handle")?,
        display_name: row.try_get("display_name")?,
    })
}

fn topic_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Topic, StoreError> {
    let id: String = row.try_get("id")?;
    Ok(Topic {
        id: TopicId::from_str(&id).map_err(|_| StoreError::CorruptData("topic id"))?,
        key: row.try_get("topic_key")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        revision: row.try_get("revision")?,
    })
}

fn generate_token() -> String {
    format!(
        "synod_{}{}",
        Uuid::now_v7().simple(),
        Uuid::now_v7().simple()
    )
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn map_constraint(error: sqlx::Error) -> StoreError {
    if matches!(&error, sqlx::Error::Database(database) if database.is_unique_violation()) {
        StoreError::Conflict
    } else {
        StoreError::Sqlx(error)
    }
}
