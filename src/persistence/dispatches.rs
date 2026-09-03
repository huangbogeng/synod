use std::{collections::HashMap, str::FromStr};

use sqlx::{Row, Sqlite, Transaction, sqlite::SqliteRow};

use crate::domain::{
    ConversationId, Dispatch, DispatchId, DispatchStatus, DispatchTarget, DispatchTargetOutcome,
    DispatchTargetSource, JobId, MentionSourceKind, Notification, NotificationId, PrincipalId,
    PrincipalKind, Run, RunConclusion, RunId, RunStatus, TopicId, TopicItemId,
};

use super::{Database, StoreError};

#[derive(Debug)]
struct PendingDispatch {
    id: DispatchId,
    topic_id: TopicId,
    source_type: String,
    source_id: String,
}

#[derive(Debug)]
struct Candidate {
    principal_id: Option<PrincipalId>,
    handle: String,
    kind: Option<PrincipalKind>,
    skip_reason: Option<&'static str>,
    sources: Vec<(String, MentionSourceKind)>,
}

impl Database {
    pub(crate) async fn resolve_next_dispatch(&self) -> Result<bool, StoreError> {
        let mut transaction = self.pool.begin().await?;
        let Some(pending) = load_next_pending(&mut transaction).await? else {
            transaction.commit().await?;
            return Ok(false);
        };
        let mentions: Vec<String> = sqlx::query_scalar(
            "SELECT handle FROM dispatch_mentions
             WHERE dispatch_id = ? ORDER BY mention_order",
        )
        .bind(pending.id.to_string())
        .fetch_all(&mut *transaction)
        .await?;
        let item_id = resolve_item_id(&mut transaction, &pending).await?;
        let candidates = expand_mentions(&mut transaction, &pending, mentions).await?;

        let mut succeeded = 0_usize;
        let mut skipped = 0_usize;
        for (order, candidate) in candidates.into_iter().enumerate() {
            let target_id = uuid::Uuid::now_v7().to_string();
            let (outcome, notification_id, run_id, skip_reason) =
                if let Some(reason) = candidate.skip_reason {
                    skipped += 1;
                    ("skipped", None, None, Some(reason))
                } else {
                    match (candidate.principal_id, candidate.kind) {
                        (Some(principal_id), Some(PrincipalKind::Human)) => {
                            let notification_id = NotificationId::new();
                            sqlx::query(
                                "INSERT INTO notifications(
                                id, dispatch_id, recipient_principal_id, kind, created_at
                             ) VALUES (?, ?, ?, 'mention', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                            )
                            .bind(notification_id.to_string())
                            .bind(pending.id.to_string())
                            .bind(principal_id.to_string())
                            .execute(&mut *transaction)
                            .await?;
                            succeeded += 1;
                            ("notified", Some(notification_id), None, None)
                        }
                        (Some(principal_id), Some(PrincipalKind::Ai)) => {
                            match queue_ai_run(&mut transaction, &pending, item_id, principal_id)
                                .await?
                            {
                                Some(run_id) => {
                                    succeeded += 1;
                                    ("queued", None, Some(run_id), None)
                                }
                                None => {
                                    skipped += 1;
                                    ("skipped", None, None, Some("ai_configuration_unavailable"))
                                }
                            }
                        }
                        _ => {
                            skipped += 1;
                            ("skipped", None, None, Some("unsupported_principal_kind"))
                        }
                    }
                };

            sqlx::query(
                "INSERT INTO dispatch_targets(
                    id, dispatch_id, principal_id, target_handle, principal_kind,
                    outcome, notification_id, run_id, skip_reason, target_order
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&target_id)
            .bind(pending.id.to_string())
            .bind(candidate.principal_id.map(|id| id.to_string()))
            .bind(&candidate.handle)
            .bind(candidate.kind.map(principal_kind_as_str))
            .bind(outcome)
            .bind(notification_id.map(|id| id.to_string()))
            .bind(run_id.map(|id| id.to_string()))
            .bind(skip_reason)
            .bind(i64::try_from(order).unwrap_or(i64::MAX))
            .execute(&mut *transaction)
            .await?;
            for (mention_handle, source_kind) in candidate.sources {
                sqlx::query(
                    "INSERT INTO dispatch_target_sources(target_id, mention_handle, source_kind)
                     VALUES (?, ?, ?)",
                )
                .bind(&target_id)
                .bind(mention_handle)
                .bind(mention_source_kind_as_str(source_kind))
                .execute(&mut *transaction)
                .await?;
            }
        }

        let status = match (succeeded, skipped) {
            (0, _) => "rejected",
            (_, 0) => "dispatched",
            _ => "partially_dispatched",
        };
        sqlx::query("UPDATE dispatches SET status = ? WHERE id = ? AND status = 'pending'")
            .bind(status)
            .bind(pending.id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub(crate) async fn get_dispatch_for(
        &self,
        actor_id: PrincipalId,
        dispatch_id: DispatchId,
    ) -> Result<Dispatch, StoreError> {
        let row = sqlx::query(
            "SELECT dispatch.id, dispatch.topic_id, dispatch.source_type, dispatch.source_id,
                    dispatch.source_revision, dispatch.author_principal_id, dispatch.status
             FROM dispatches AS dispatch
             JOIN topic_memberships AS membership ON membership.topic_id = dispatch.topic_id
             WHERE dispatch.id = ? AND membership.principal_id = ?",
        )
        .bind(dispatch_id.to_string())
        .bind(actor_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        let mentions = sqlx::query_scalar(
            "SELECT handle FROM dispatch_mentions WHERE dispatch_id = ? ORDER BY mention_order",
        )
        .bind(dispatch_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let target_rows = sqlx::query(
            "SELECT id, principal_id, target_handle, principal_kind, outcome,
                    notification_id, run_id, skip_reason
             FROM dispatch_targets WHERE dispatch_id = ? ORDER BY target_order",
        )
        .bind(dispatch_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let mut targets = Vec::with_capacity(target_rows.len());
        for target_row in target_rows {
            let target_id: String = target_row.try_get("id")?;
            let source_rows = sqlx::query(
                "SELECT mention_handle, source_kind FROM dispatch_target_sources
                 WHERE target_id = ? ORDER BY rowid",
            )
            .bind(target_id)
            .fetch_all(&self.pool)
            .await?;
            let sources = source_rows
                .iter()
                .map(dispatch_target_source_from_row)
                .collect::<Result<Vec<_>, _>>()?;
            targets.push(dispatch_target_from_row(&target_row, sources)?);
        }
        dispatch_from_row(&row, mentions, targets)
    }

    pub(crate) async fn get_run_for(
        &self,
        actor_id: PrincipalId,
        run_id: RunId,
    ) -> Result<Run, StoreError> {
        let row = sqlx::query(
            "SELECT run.id, run.dispatch_id, run.topic_id, run.item_id, run.ai_principal_id,
                    run.conversation_id, run.identity_prompt_version, run.model_id,
                    run.model_parameters_json, run.context_snapshot_id, run.status,
                    run.conclusion, run.retry_of_run_id
             FROM runs AS run
             JOIN topic_memberships AS membership ON membership.topic_id = run.topic_id
             WHERE run.id = ? AND membership.principal_id = ?",
        )
        .bind(run_id.to_string())
        .bind(actor_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        run_from_row(&row)
    }

    pub(crate) async fn list_runs_for(
        &self,
        actor_id: PrincipalId,
        topic_id: TopicId,
    ) -> Result<Vec<Run>, StoreError> {
        let rows = sqlx::query(
            "SELECT run.id, run.dispatch_id, run.topic_id, run.item_id, run.ai_principal_id,
                    run.conversation_id, run.identity_prompt_version, run.model_id,
                    run.model_parameters_json, run.context_snapshot_id, run.status,
                    run.conclusion, run.retry_of_run_id
             FROM runs AS run
             JOIN topic_memberships AS membership ON membership.topic_id = run.topic_id
             WHERE run.topic_id = ? AND membership.principal_id = ?
             ORDER BY run.created_at DESC, run.id DESC",
        )
        .bind(topic_id.to_string())
        .bind(actor_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(run_from_row).collect()
    }

    pub(crate) async fn list_notifications_for(
        &self,
        actor_id: PrincipalId,
    ) -> Result<Vec<Notification>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, dispatch_id, recipient_principal_id, kind, read_at
             FROM notifications WHERE recipient_principal_id = ?
             ORDER BY created_at DESC, id DESC",
        )
        .bind(actor_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(notification_from_row).collect()
    }
}

async fn load_next_pending(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<Option<PendingDispatch>, StoreError> {
    let row = sqlx::query(
        "SELECT id, topic_id, source_type, source_id
         FROM dispatches WHERE status = 'pending' ORDER BY created_at, id LIMIT 1",
    )
    .fetch_optional(&mut **transaction)
    .await?;
    row.map(|row| {
        Ok(PendingDispatch {
            id: parse_id(&row, "id", "dispatch id")?,
            topic_id: parse_id(&row, "topic_id", "topic id")?,
            source_type: row.try_get("source_type")?,
            source_id: row.try_get("source_id")?,
        })
    })
    .transpose()
}

async fn resolve_item_id(
    transaction: &mut Transaction<'_, Sqlite>,
    pending: &PendingDispatch,
) -> Result<TopicItemId, StoreError> {
    let raw: Option<String> = if pending.source_type == "comment" {
        sqlx::query_scalar("SELECT item_id FROM comments WHERE id = ?")
            .bind(&pending.source_id)
            .fetch_optional(&mut **transaction)
            .await?
    } else {
        sqlx::query_scalar("SELECT id FROM topic_items WHERE id = ? AND topic_id = ?")
            .bind(&pending.source_id)
            .bind(pending.topic_id.to_string())
            .fetch_optional(&mut **transaction)
            .await?
    };
    raw.ok_or(StoreError::CorruptData("dispatch source"))?
        .parse()
        .map_err(|_| StoreError::CorruptData("item id"))
}

async fn expand_mentions(
    transaction: &mut Transaction<'_, Sqlite>,
    pending: &PendingDispatch,
    mentions: Vec<String>,
) -> Result<Vec<Candidate>, StoreError> {
    let mut candidates = Vec::<Candidate>::new();
    let mut by_principal = HashMap::<PrincipalId, usize>::new();

    for mention in mentions {
        let principal = sqlx::query(
            "SELECT principal.id, principal.handle, principal.kind, principal.active,
                    membership.principal_id AS topic_member_id
             FROM principals AS principal
             JOIN topic_memberships AS membership ON membership.principal_id = principal.id
             WHERE membership.topic_id = ? AND principal.handle = ? COLLATE NOCASE",
        )
        .bind(pending.topic_id.to_string())
        .bind(&mention)
        .fetch_optional(&mut **transaction)
        .await?;
        if let Some(row) = principal {
            add_principal_candidate(
                &mut candidates,
                &mut by_principal,
                principal_candidate(&row, &mention, MentionSourceKind::Direct)?,
            );
            continue;
        }

        let team_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM teams WHERE topic_id = ? AND handle = ? COLLATE NOCASE",
        )
        .bind(pending.topic_id.to_string())
        .bind(&mention)
        .fetch_optional(&mut **transaction)
        .await?;
        if let Some(team_id) = team_id {
            let rows = sqlx::query(
                "SELECT principal.id, principal.handle, principal.kind, principal.active,
                        membership.principal_id AS topic_member_id
                 FROM team_members AS member
                 JOIN principals AS principal ON principal.id = member.principal_id
                 LEFT JOIN topic_memberships AS membership
                   ON membership.principal_id = principal.id AND membership.topic_id = ?
                 WHERE member.team_id = ? ORDER BY member.created_at, principal.id",
            )
            .bind(pending.topic_id.to_string())
            .bind(team_id)
            .fetch_all(&mut **transaction)
            .await?;
            if rows.is_empty() {
                candidates.push(unresolved_candidate(
                    mention,
                    MentionSourceKind::Team,
                    "team_has_no_members",
                ));
            } else {
                for row in rows {
                    add_principal_candidate(
                        &mut candidates,
                        &mut by_principal,
                        principal_candidate(&row, &mention, MentionSourceKind::Team)?,
                    );
                }
            }
        } else {
            candidates.push(unresolved_candidate(
                mention,
                MentionSourceKind::Direct,
                "unknown_handle",
            ));
        }
    }
    Ok(candidates)
}

fn principal_candidate(
    row: &SqliteRow,
    mention: &str,
    source_kind: MentionSourceKind,
) -> Result<Candidate, StoreError> {
    let kind_raw: String = row.try_get("kind")?;
    let kind = principal_kind_from_stored(&kind_raw)?;
    let active: bool = row.try_get("active")?;
    let topic_member_id: Option<String> = row.try_get("topic_member_id")?;
    Ok(Candidate {
        principal_id: Some(parse_id(row, "id", "principal id")?),
        handle: row.try_get("handle")?,
        kind: Some(kind),
        skip_reason: if !active {
            Some("principal_inactive")
        } else if topic_member_id.is_none() {
            Some("not_topic_member")
        } else {
            None
        },
        sources: vec![(mention.to_owned(), source_kind)],
    })
}

fn unresolved_candidate(
    handle: String,
    source_kind: MentionSourceKind,
    reason: &'static str,
) -> Candidate {
    Candidate {
        principal_id: None,
        handle: handle.clone(),
        kind: None,
        skip_reason: Some(reason),
        sources: vec![(handle, source_kind)],
    }
}

fn add_principal_candidate(
    candidates: &mut Vec<Candidate>,
    by_principal: &mut HashMap<PrincipalId, usize>,
    candidate: Candidate,
) {
    let principal_id = candidate
        .principal_id
        .expect("principal candidate has an id");
    if let Some(index) = by_principal.get(&principal_id).copied() {
        candidates[index].sources.extend(candidate.sources);
    } else {
        by_principal.insert(principal_id, candidates.len());
        candidates.push(candidate);
    }
}

async fn queue_ai_run(
    transaction: &mut Transaction<'_, Sqlite>,
    pending: &PendingDispatch,
    item_id: TopicItemId,
    ai_principal_id: PrincipalId,
) -> Result<Option<RunId>, StoreError> {
    let profile = sqlx::query(
        "SELECT profile.identity_prompt_version, profile.default_model_id,
                profile.execution_defaults_json, model.defaults_json AS model_defaults_json
         FROM ai_profiles AS profile
         JOIN models AS model ON model.id = profile.default_model_id
         JOIN providers AS provider ON provider.id = model.provider_id
         WHERE profile.principal_id = ? AND model.enabled = 1 AND provider.enabled = 1",
    )
    .bind(ai_principal_id.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(profile) = profile else {
        return Ok(None);
    };
    let identity_prompt_version: i64 = profile.try_get("identity_prompt_version")?;
    let model_id: String = profile.try_get("default_model_id")?;
    let model_defaults: String = profile.try_get("model_defaults_json")?;
    let member_defaults: String = profile.try_get("execution_defaults_json")?;
    let model_parameters = merge_model_parameters(&model_defaults, &member_defaults)?;

    let proposed_conversation_id = ConversationId::new().to_string();
    sqlx::query(
        "INSERT INTO conversations(
            id, topic_id, item_id, ai_principal_id, created_at, updated_at
         ) VALUES (?, ?, ?, ?,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT(item_id, ai_principal_id) DO NOTHING",
    )
    .bind(&proposed_conversation_id)
    .bind(pending.topic_id.to_string())
    .bind(item_id.to_string())
    .bind(ai_principal_id.to_string())
    .execute(&mut **transaction)
    .await?;
    let conversation_id: String = sqlx::query_scalar(
        "SELECT id FROM conversations WHERE item_id = ? AND ai_principal_id = ?",
    )
    .bind(item_id.to_string())
    .bind(ai_principal_id.to_string())
    .fetch_one(&mut **transaction)
    .await?;

    let run_id = RunId::new();
    sqlx::query(
        "INSERT INTO runs(
            id, dispatch_id, topic_id, item_id, ai_principal_id, conversation_id,
            identity_prompt_version, model_id, model_parameters_json, status, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'queued',
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(run_id.to_string())
    .bind(pending.id.to_string())
    .bind(pending.topic_id.to_string())
    .bind(item_id.to_string())
    .bind(ai_principal_id.to_string())
    .bind(&conversation_id)
    .bind(identity_prompt_version)
    .bind(model_id)
    .bind(model_parameters)
    .execute(&mut **transaction)
    .await?;

    let job_id = JobId::new();
    let payload = serde_json::json!({"run_id": run_id}).to_string();
    sqlx::query(
        "INSERT INTO jobs(
            id, kind, payload, state, available_at, created_at, updated_at
         ) VALUES (?, 'run.execute', ?, 'queued',
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(job_id.to_string())
    .bind(payload)
    .execute(&mut **transaction)
    .await?;
    Ok(Some(run_id))
}

fn dispatch_from_row(
    row: &SqliteRow,
    mentions: Vec<String>,
    targets: Vec<DispatchTarget>,
) -> Result<Dispatch, StoreError> {
    let status: String = row.try_get("status")?;
    Ok(Dispatch {
        id: parse_id(row, "id", "dispatch id")?,
        topic_id: parse_id(row, "topic_id", "topic id")?,
        source_type: row.try_get("source_type")?,
        source_id: row.try_get("source_id")?,
        source_revision: row.try_get("source_revision")?,
        author_id: parse_id(row, "author_principal_id", "author id")?,
        status: dispatch_status_from_stored(&status)?,
        mentions,
        targets,
    })
}

fn dispatch_target_from_row(
    row: &SqliteRow,
    sources: Vec<DispatchTargetSource>,
) -> Result<DispatchTarget, StoreError> {
    let principal_kind: Option<String> = row.try_get("principal_kind")?;
    let outcome: String = row.try_get("outcome")?;
    Ok(DispatchTarget {
        principal_id: parse_optional_id(row, "principal_id", "principal id")?,
        handle: row.try_get("target_handle")?,
        principal_kind: principal_kind
            .as_deref()
            .map(principal_kind_from_stored)
            .transpose()?,
        outcome: match outcome.as_str() {
            "notified" => DispatchTargetOutcome::Notified,
            "queued" => DispatchTargetOutcome::Queued,
            "skipped" => DispatchTargetOutcome::Skipped,
            _ => return Err(StoreError::CorruptData("dispatch target outcome")),
        },
        notification_id: parse_optional_id(row, "notification_id", "notification id")?,
        run_id: parse_optional_id(row, "run_id", "run id")?,
        skip_reason: row.try_get("skip_reason")?,
        sources,
    })
}

fn dispatch_target_source_from_row(row: &SqliteRow) -> Result<DispatchTargetSource, StoreError> {
    let kind: String = row.try_get("source_kind")?;
    Ok(DispatchTargetSource {
        mention_handle: row.try_get("mention_handle")?,
        source_kind: match kind.as_str() {
            "direct" => MentionSourceKind::Direct,
            "team" => MentionSourceKind::Team,
            _ => return Err(StoreError::CorruptData("mention source kind")),
        },
    })
}

fn run_from_row(row: &SqliteRow) -> Result<Run, StoreError> {
    let status: String = row.try_get("status")?;
    let conclusion: Option<String> = row.try_get("conclusion")?;
    Ok(Run {
        id: parse_id(row, "id", "run id")?,
        dispatch_id: parse_id(row, "dispatch_id", "dispatch id")?,
        topic_id: parse_id(row, "topic_id", "topic id")?,
        item_id: parse_id(row, "item_id", "item id")?,
        ai_member_id: parse_id(row, "ai_principal_id", "ai principal id")?,
        conversation_id: parse_id(row, "conversation_id", "conversation id")?,
        identity_prompt_version: row.try_get("identity_prompt_version")?,
        model_id: parse_id(row, "model_id", "model id")?,
        model_parameters: parse_json_value(row, "model_parameters_json", "model parameters")?,
        context_snapshot_id: parse_optional_id(row, "context_snapshot_id", "context snapshot id")?,
        status: match status.as_str() {
            "queued" => RunStatus::Queued,
            "in_progress" => RunStatus::InProgress,
            "completed" => RunStatus::Completed,
            _ => return Err(StoreError::CorruptData("run status")),
        },
        conclusion: conclusion
            .as_deref()
            .map(run_conclusion_from_stored)
            .transpose()?,
        retry_of_run_id: parse_optional_id(row, "retry_of_run_id", "retry run id")?,
    })
}

fn merge_model_parameters(
    model_defaults: &str,
    member_defaults: &str,
) -> Result<String, StoreError> {
    let mut merged: serde_json::Value = serde_json::from_str(model_defaults)
        .map_err(|_| StoreError::CorruptData("model defaults"))?;
    let member: serde_json::Value = serde_json::from_str(member_defaults)
        .map_err(|_| StoreError::CorruptData("AI Member execution defaults"))?;
    let merged = merged
        .as_object_mut()
        .ok_or(StoreError::CorruptData("model defaults"))?;
    let member = member
        .as_object()
        .ok_or(StoreError::CorruptData("AI Member execution defaults"))?;
    for (key, value) in member {
        merged.insert(key.clone(), value.clone());
    }
    Ok(serde_json::Value::Object(merged.clone()).to_string())
}

fn parse_json_value(
    row: &SqliteRow,
    column: &str,
    label: &'static str,
) -> Result<serde_json::Value, StoreError> {
    let raw: String = row.try_get(column)?;
    serde_json::from_str(&raw).map_err(|_| StoreError::CorruptData(label))
}

fn notification_from_row(row: &SqliteRow) -> Result<Notification, StoreError> {
    let read_at: Option<String> = row.try_get("read_at")?;
    Ok(Notification {
        id: parse_id(row, "id", "notification id")?,
        dispatch_id: parse_id(row, "dispatch_id", "dispatch id")?,
        recipient_id: parse_id(row, "recipient_principal_id", "recipient id")?,
        kind: row.try_get("kind")?,
        read: read_at.is_some(),
    })
}

fn dispatch_status_from_stored(value: &str) -> Result<DispatchStatus, StoreError> {
    match value {
        "pending" => Ok(DispatchStatus::Pending),
        "dispatched" => Ok(DispatchStatus::Dispatched),
        "partially_dispatched" => Ok(DispatchStatus::PartiallyDispatched),
        "rejected" => Ok(DispatchStatus::Rejected),
        _ => Err(StoreError::CorruptData("dispatch status")),
    }
}

fn principal_kind_from_stored(value: &str) -> Result<PrincipalKind, StoreError> {
    match value {
        "human" => Ok(PrincipalKind::Human),
        "ai" => Ok(PrincipalKind::Ai),
        "caller" => Ok(PrincipalKind::Caller),
        "system" => Ok(PrincipalKind::System),
        _ => Err(StoreError::CorruptData("principal kind")),
    }
}

const fn principal_kind_as_str(value: PrincipalKind) -> &'static str {
    match value {
        PrincipalKind::Human => "human",
        PrincipalKind::Ai => "ai",
        PrincipalKind::Caller => "caller",
        PrincipalKind::System => "system",
    }
}

const fn mention_source_kind_as_str(value: MentionSourceKind) -> &'static str {
    match value {
        MentionSourceKind::Direct => "direct",
        MentionSourceKind::Team => "team",
    }
}

fn run_conclusion_from_stored(value: &str) -> Result<RunConclusion, StoreError> {
    match value {
        "success" => Ok(RunConclusion::Success),
        "failure" => Ok(RunConclusion::Failure),
        "cancelled" => Ok(RunConclusion::Cancelled),
        "timed_out" => Ok(RunConclusion::TimedOut),
        "skipped" => Ok(RunConclusion::Skipped),
        "neutral" => Ok(RunConclusion::Neutral),
        _ => Err(StoreError::CorruptData("run conclusion")),
    }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        domain::{
            DispatchStatus, DispatchTargetOutcome, MembershipRole, PrincipalId, ProviderAdapter,
        },
        services::{AdminService, IssueService, MembershipService, TopicService},
    };

    use super::*;

    #[tokio::test]
    async fn team_dispatch_deduplicates_targets_and_queues_ai_work() {
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
                String::new(),
            )
            .await
            .unwrap();

        let bob_id = PrincipalId::new();
        sqlx::query(
            "INSERT INTO principals(id, kind, handle, display_name, created_at)
             VALUES (?, 'human', 'bob', 'Bob', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(bob_id.to_string())
        .execute(database.pool())
        .await
        .unwrap();

        let admin = AdminService::new(database.clone());
        let provider = admin
            .create_provider(
                &alice,
                "Test".to_owned(),
                ProviderAdapter::OpenaiCompatible,
                "https://api.deepseek.com".to_owned(),
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
                serde_json::json!({}),
                serde_json::json!({}),
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
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let memberships = MembershipService::new(database.clone());
        memberships
            .put_topic_member(&alice, topic.id, bob_id, MembershipRole::Contribute)
            .await
            .unwrap();
        memberships
            .put_topic_member(
                &alice,
                topic.id,
                architect.principal.id,
                MembershipRole::Contribute,
            )
            .await
            .unwrap();
        let team = memberships
            .create_team(
                &alice,
                topic.id,
                "reviewers".to_owned(),
                "Reviewers".to_owned(),
            )
            .await
            .unwrap();
        memberships
            .put_team_member(&alice, team.id, bob_id)
            .await
            .unwrap();
        memberships
            .put_team_member(&alice, team.id, architect.principal.id)
            .await
            .unwrap();

        let creation = IssueService::new(database.clone())
            .create_issue(
                &alice,
                topic.id,
                "code_audit".to_owned(),
                "Audit dispatch".to_owned(),
                "@reviewers inspect this. @architect focus on boundaries. @nobody verify."
                    .to_owned(),
                None,
            )
            .await
            .unwrap();
        let dispatch_id = creation.dispatch_id.unwrap();

        assert!(database.resolve_next_dispatch().await.unwrap());
        assert!(!database.resolve_next_dispatch().await.unwrap());

        let dispatch = database
            .get_dispatch_for(alice.id, dispatch_id)
            .await
            .unwrap();
        assert_eq!(dispatch.status, DispatchStatus::PartiallyDispatched);
        assert_eq!(dispatch.targets.len(), 3);

        let bob = dispatch
            .targets
            .iter()
            .find(|target| target.handle == "bob")
            .unwrap();
        assert_eq!(bob.outcome, DispatchTargetOutcome::Notified);
        assert!(bob.notification_id.is_some());

        let ai = dispatch
            .targets
            .iter()
            .find(|target| target.handle == "architect")
            .unwrap();
        assert_eq!(ai.outcome, DispatchTargetOutcome::Queued);
        assert_eq!(ai.sources.len(), 2);
        let run = database
            .get_run_for(alice.id, ai.run_id.unwrap())
            .await
            .unwrap();
        assert_eq!(run.status, RunStatus::Queued);
        assert_eq!(run.item_id, creation.value.id);

        let unknown = dispatch
            .targets
            .iter()
            .find(|target| target.handle == "nobody")
            .unwrap();
        assert_eq!(unknown.outcome, DispatchTargetOutcome::Skipped);
        assert_eq!(unknown.skip_reason.as_deref(), Some("unknown_handle"));

        assert_eq!(
            database.list_notifications_for(bob_id).await.unwrap().len(),
            1
        );
        let run_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM runs")
            .fetch_one(database.pool())
            .await
            .unwrap();
        let job_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM jobs WHERE kind = 'run.execute' AND state = 'queued'",
        )
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(run_count, 1);
        assert_eq!(job_count, 1);

        let suggested = IssueService::new(database.clone())
            .create_comment(
                &architect.principal,
                creation.value.id,
                crate::domain::CommentKind::Discussion,
                "@alice should decide this.".to_owned(),
                None,
            )
            .await
            .unwrap();
        assert!(suggested.dispatch_id.is_none());
    }
}
