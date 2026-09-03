use std::str::FromStr;

use sqlx::{Row, Sqlite, Transaction, sqlite::SqliteRow};

use crate::domain::{
    Comment, CommentId, CreateComment, CreateIssue, DispatchId, Issue, IssueState, IssueType,
    MembershipRole, Principal, PrincipalId, PrincipalKind, TopicId, TopicItemId, parse_mentions,
};

use super::{Database, StoreError};

pub struct StoredCreation<T> {
    pub value: T,
    pub dispatch_id: Option<DispatchId>,
}

impl Database {
    pub(crate) async fn list_issue_types(&self) -> Result<Vec<IssueType>, StoreError> {
        let rows = sqlx::query(
            "SELECT type_key, display_name, description FROM issue_types ORDER BY rowid",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(issue_type_from_row).collect()
    }

    pub(crate) async fn insert_issue(
        &self,
        actor: &Principal,
        topic_id: TopicId,
        input: &CreateIssue,
    ) -> Result<StoredCreation<Issue>, StoreError> {
        let item_id = TopicItemId::new();
        let mentions = parse_mentions(&input.body);
        let mut transaction = self.pool.begin().await?;
        require_contribution(&mut transaction, topic_id, actor.id).await?;

        let type_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM issue_types WHERE type_key = ?")
                .bind(&input.issue_type)
                .fetch_one(&mut *transaction)
                .await?;
        if type_exists != 1 {
            return Err(StoreError::InvalidReference("unknown issue type"));
        }

        if let Some(parent_id) = input.parent_issue_id {
            let parent_topic: Option<String> = sqlx::query_scalar(
                "SELECT item.topic_id
                 FROM topic_items AS item
                 JOIN issues AS issue ON issue.item_id = item.id
                 WHERE item.id = ?",
            )
            .bind(parent_id.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
            if parent_topic.as_deref() != Some(topic_id.to_string().as_str()) {
                return Err(StoreError::InvalidReference(
                    "parent issue must belong to the same topic",
                ));
            }
        }

        let number: i64 = sqlx::query_scalar(
            "UPDATE topics
             SET next_issue_number = next_issue_number + 1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?
             RETURNING next_issue_number - 1",
        )
        .bind(topic_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO topic_items(
                id, topic_id, number, kind, title, author_principal_id,
                created_at, updated_at
             ) VALUES (?, ?, ?, 'issue', ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(item_id.to_string())
        .bind(topic_id.to_string())
        .bind(number)
        .bind(&input.title)
        .bind(actor.id.to_string())
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO issues(item_id, type_key, state, body, parent_issue_item_id)
             VALUES (?, ?, 'open', ?, ?)",
        )
        .bind(item_id.to_string())
        .bind(&input.issue_type)
        .bind(&input.body)
        .bind(input.parent_issue_id.map(|id| id.to_string()))
        .execute(&mut *transaction)
        .await?;

        insert_activity_event(
            &mut transaction,
            topic_id,
            Some(item_id),
            "issue.created",
            actor.id,
            "issue",
            &item_id.to_string(),
        )
        .await?;
        let dispatch_id = insert_dispatch(
            &mut transaction,
            topic_id,
            "issue",
            &item_id.to_string(),
            1,
            actor,
            &mentions,
        )
        .await?;

        transaction.commit().await?;
        Ok(StoredCreation {
            value: Issue {
                id: item_id,
                topic_id,
                number,
                issue_type: input.issue_type.clone(),
                state: IssueState::Open,
                title: input.title.clone(),
                body: input.body.clone(),
                parent_issue_id: input.parent_issue_id,
                author_id: actor.id,
                revision: 1,
            },
            dispatch_id,
        })
    }

    pub(crate) async fn list_issues_for(
        &self,
        actor_id: PrincipalId,
        topic_id: TopicId,
    ) -> Result<Vec<Issue>, StoreError> {
        require_read(&self.pool, topic_id, actor_id).await?;
        let rows = sqlx::query(
            "SELECT item.id, item.topic_id, item.number, item.title,
                    item.author_principal_id, item.revision,
                    issue.type_key, issue.state, issue.body, issue.parent_issue_item_id
             FROM topic_items AS item
             JOIN issues AS issue ON issue.item_id = item.id
             WHERE item.topic_id = ?
             ORDER BY item.number DESC",
        )
        .bind(topic_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(issue_from_row).collect()
    }

    pub(crate) async fn get_issue_for(
        &self,
        actor_id: PrincipalId,
        issue_id: TopicItemId,
    ) -> Result<Issue, StoreError> {
        let row = sqlx::query(
            "SELECT item.id, item.topic_id, item.number, item.title,
                    item.author_principal_id, item.revision,
                    issue.type_key, issue.state, issue.body, issue.parent_issue_item_id
             FROM topic_items AS item
             JOIN issues AS issue ON issue.item_id = item.id
             JOIN topic_memberships AS membership ON membership.topic_id = item.topic_id
             WHERE item.id = ? AND membership.principal_id = ?",
        )
        .bind(issue_id.to_string())
        .bind(actor_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        issue_from_row(&row)
    }

    pub(crate) async fn insert_issue_comment(
        &self,
        actor: &Principal,
        issue_id: TopicItemId,
        input: &CreateComment,
    ) -> Result<StoredCreation<Comment>, StoreError> {
        let comment_id = CommentId::new();
        let mentions = parse_mentions(&input.body);
        let mut transaction = self.pool.begin().await?;

        let topic_id_raw: Option<String> = sqlx::query_scalar(
            "SELECT item.topic_id FROM topic_items AS item
             JOIN issues AS issue ON issue.item_id = item.id
             WHERE item.id = ?",
        )
        .bind(issue_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        let topic_id = topic_id_raw
            .ok_or(StoreError::NotFound)?
            .parse::<TopicId>()
            .map_err(|_| StoreError::CorruptData("topic id"))?;
        require_contribution(&mut transaction, topic_id, actor.id).await?;

        if let Some(reply_id) = input.reply_to_comment_id {
            let reply_item: Option<String> =
                sqlx::query_scalar("SELECT item_id FROM comments WHERE id = ?")
                    .bind(reply_id.to_string())
                    .fetch_optional(&mut *transaction)
                    .await?;
            if reply_item.as_deref() != Some(issue_id.to_string().as_str()) {
                return Err(StoreError::InvalidReference(
                    "reply must reference a comment on the same issue",
                ));
            }
        }

        sqlx::query(
            "INSERT INTO comments(
                id, item_id, author_principal_id, kind, body, reply_to_comment_id,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(comment_id.to_string())
        .bind(issue_id.to_string())
        .bind(actor.id.to_string())
        .bind(input.kind.as_str())
        .bind(&input.body)
        .bind(input.reply_to_comment_id.map(|id| id.to_string()))
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO comment_revisions(
                comment_id, revision, body, editor_principal_id, created_at
             ) VALUES (?, 1, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(comment_id.to_string())
        .bind(&input.body)
        .bind(actor.id.to_string())
        .execute(&mut *transaction)
        .await?;

        insert_activity_event(
            &mut transaction,
            topic_id,
            Some(issue_id),
            "comment.created",
            actor.id,
            "comment",
            &comment_id.to_string(),
        )
        .await?;
        let dispatch_id = insert_dispatch(
            &mut transaction,
            topic_id,
            "comment",
            &comment_id.to_string(),
            1,
            actor,
            &mentions,
        )
        .await?;

        transaction.commit().await?;
        Ok(StoredCreation {
            value: Comment {
                id: comment_id,
                item_id: issue_id,
                author_id: actor.id,
                kind: input.kind,
                body: input.body.clone(),
                revision: 1,
                reply_to_comment_id: input.reply_to_comment_id,
            },
            dispatch_id,
        })
    }

    pub(crate) async fn list_issue_comments_for(
        &self,
        actor_id: PrincipalId,
        issue_id: TopicItemId,
    ) -> Result<Vec<Comment>, StoreError> {
        let rows = sqlx::query(
            "SELECT comment.id, comment.item_id, comment.author_principal_id,
                    comment.kind, comment.body, comment.revision, comment.reply_to_comment_id
             FROM comments AS comment
             JOIN topic_items AS item ON item.id = comment.item_id
             JOIN issues AS issue ON issue.item_id = item.id
             JOIN topic_memberships AS membership ON membership.topic_id = item.topic_id
             WHERE item.id = ? AND membership.principal_id = ?
               AND comment.tombstoned_at IS NULL
             ORDER BY comment.created_at, comment.id",
        )
        .bind(issue_id.to_string())
        .bind(actor_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(comment_from_row).collect()
    }
}

async fn require_read(
    pool: &sqlx::SqlitePool,
    topic_id: TopicId,
    principal_id: PrincipalId,
) -> Result<MembershipRole, StoreError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM topic_memberships WHERE topic_id = ? AND principal_id = ?",
    )
    .bind(topic_id.to_string())
    .bind(principal_id.to_string())
    .fetch_optional(pool)
    .await?;
    role.as_deref()
        .and_then(MembershipRole::from_stored)
        .ok_or(StoreError::NotFound)
}

async fn require_contribution(
    transaction: &mut Transaction<'_, Sqlite>,
    topic_id: TopicId,
    principal_id: PrincipalId,
) -> Result<(), StoreError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM topic_memberships WHERE topic_id = ? AND principal_id = ?",
    )
    .bind(topic_id.to_string())
    .bind(principal_id.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    let role = role
        .as_deref()
        .and_then(MembershipRole::from_stored)
        .ok_or(StoreError::NotFound)?;
    if !role.can_contribute() {
        return Err(StoreError::PermissionDenied);
    }
    Ok(())
}

pub(super) async fn insert_activity_event(
    transaction: &mut Transaction<'_, Sqlite>,
    topic_id: TopicId,
    item_id: Option<TopicItemId>,
    event_type: &str,
    actor_id: PrincipalId,
    subject_type: &str,
    subject_id: &str,
) -> Result<(), sqlx::Error> {
    let sequence: i64 = sqlx::query_scalar(
        "UPDATE topics SET next_event_sequence = next_event_sequence + 1
         WHERE id = ? RETURNING next_event_sequence - 1",
    )
    .bind(topic_id.to_string())
    .fetch_one(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO activity_events(
            id, topic_id, sequence, item_id, event_type, actor_principal_id,
            subject_type, subject_id, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(topic_id.to_string())
    .bind(sequence)
    .bind(item_id.map(|id| id.to_string()))
    .bind(event_type)
    .bind(actor_id.to_string())
    .bind(subject_type)
    .bind(subject_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_dispatch(
    transaction: &mut Transaction<'_, Sqlite>,
    topic_id: TopicId,
    source_type: &str,
    source_id: &str,
    source_revision: i64,
    author: &Principal,
    mentions: &[String],
) -> Result<Option<DispatchId>, sqlx::Error> {
    if mentions.is_empty() || author.kind == PrincipalKind::Ai {
        return Ok(None);
    }
    let dispatch_id = DispatchId::new();
    sqlx::query(
        "INSERT INTO dispatches(
            id, topic_id, source_type, source_id, source_revision,
            author_principal_id, status, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, 'pending', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(dispatch_id.to_string())
    .bind(topic_id.to_string())
    .bind(source_type)
    .bind(source_id)
    .bind(source_revision)
    .bind(author.id.to_string())
    .execute(&mut **transaction)
    .await?;
    for (order, handle) in mentions.iter().enumerate() {
        sqlx::query(
            "INSERT INTO dispatch_mentions(dispatch_id, handle, mention_order) VALUES (?, ?, ?)",
        )
        .bind(dispatch_id.to_string())
        .bind(handle)
        .bind(i64::try_from(order).unwrap_or(i64::MAX))
        .execute(&mut **transaction)
        .await?;
    }
    Ok(Some(dispatch_id))
}

fn issue_type_from_row(row: &SqliteRow) -> Result<IssueType, StoreError> {
    Ok(IssueType {
        key: row.try_get("type_key")?,
        display_name: row.try_get("display_name")?,
        description: row.try_get("description")?,
    })
}

fn issue_from_row(row: &SqliteRow) -> Result<Issue, StoreError> {
    let state: String = row.try_get("state")?;
    Ok(Issue {
        id: parse_id(row, "id", "issue id")?,
        topic_id: parse_id(row, "topic_id", "topic id")?,
        number: row.try_get("number")?,
        issue_type: row.try_get("type_key")?,
        state: match state.as_str() {
            "open" => IssueState::Open,
            "closed" => IssueState::Closed,
            _ => return Err(StoreError::CorruptData("issue state")),
        },
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        parent_issue_id: parse_optional_id(row, "parent_issue_item_id", "parent issue id")?,
        author_id: parse_id(row, "author_principal_id", "author id")?,
        revision: row.try_get("revision")?,
    })
}

fn comment_from_row(row: &SqliteRow) -> Result<Comment, StoreError> {
    let kind: String = row.try_get("kind")?;
    Ok(Comment {
        id: parse_id(row, "id", "comment id")?,
        item_id: parse_id(row, "item_id", "item id")?,
        author_id: parse_id(row, "author_principal_id", "author id")?,
        kind: match kind.as_str() {
            "discussion" => crate::domain::CommentKind::Discussion,
            "direction" => crate::domain::CommentKind::Direction,
            "evidence" => crate::domain::CommentKind::Evidence,
            "progress" => crate::domain::CommentKind::Progress,
            "result" => crate::domain::CommentKind::Result,
            _ => return Err(StoreError::CorruptData("comment kind")),
        },
        body: row.try_get("body")?,
        revision: row.try_get("revision")?,
        reply_to_comment_id: parse_optional_id(row, "reply_to_comment_id", "reply comment id")?,
    })
}

fn parse_id<T>(row: &SqliteRow, column: &str, label: &'static str) -> Result<T, StoreError>
where
    T: FromStr,
{
    let raw: String = row.try_get(column)?;
    raw.parse().map_err(|_| StoreError::CorruptData(label))
}

fn parse_optional_id<T>(
    row: &SqliteRow,
    column: &str,
    label: &'static str,
) -> Result<Option<T>, StoreError>
where
    T: FromStr,
{
    let raw: Option<String> = row.try_get(column)?;
    raw.map(|value| value.parse().map_err(|_| StoreError::CorruptData(label)))
        .transpose()
}
