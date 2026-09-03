use std::{collections::BTreeMap, str::FromStr};

use sqlx::{Row, Sqlite, Transaction};

use crate::domain::{
    AiMember, MembershipRole, Model, ModelId, ModelInput, Principal, PrincipalId, PrincipalKind,
    Provider, ProviderAdapter, ProviderId, Team, TeamId, TopicId, TopicMember,
};

use super::{Database, StoreError, issues::insert_activity_event};

impl Database {
    pub(crate) async fn require_server_admin(
        &self,
        principal_id: PrincipalId,
    ) -> Result<(), StoreError> {
        let allowed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM server_state AS state
             JOIN principals AS principal ON principal.id = state.bootstrap_principal_id
             WHERE state.singleton_id = 1 AND principal.id = ?
               AND principal.kind = 'human' AND principal.active = 1",
        )
        .bind(principal_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        if allowed == 1 {
            Ok(())
        } else {
            Err(StoreError::PermissionDenied)
        }
    }

    pub(crate) async fn insert_provider(
        &self,
        actor_id: PrincipalId,
        name: &str,
        adapter: ProviderAdapter,
        base_url: &str,
        credential_ref: &str,
    ) -> Result<Provider, StoreError> {
        self.require_server_admin(actor_id).await?;
        let id = ProviderId::new();
        sqlx::query(
            "INSERT INTO providers(
                id, name, adapter, base_url, credential_ref, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(id.to_string())
        .bind(name)
        .bind(adapter.as_str())
        .bind(base_url)
        .bind(credential_ref)
        .execute(&self.pool)
        .await
        .map_err(super::store::map_constraint)?;
        Ok(Provider {
            id,
            name: name.to_owned(),
            adapter,
            base_url: base_url.to_owned(),
            credential_configured: !credential_ref.is_empty(),
            enabled: true,
        })
    }

    pub(crate) async fn insert_provider_with_secret(
        &self,
        actor_id: PrincipalId,
        name: &str,
        adapter: ProviderAdapter,
        base_url: &str,
        secret: &str,
    ) -> Result<Provider, StoreError> {
        self.require_server_admin(actor_id).await?;
        let id = ProviderId::new();
        let credential_ref = format!("secret://{id}");
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO providers(
                id, name, adapter, base_url, credential_ref, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(id.to_string())
        .bind(name)
        .bind(adapter.as_str())
        .bind(base_url)
        .bind(&credential_ref)
        .execute(&mut *transaction)
        .await
        .map_err(super::store::map_constraint)?;
        sqlx::query(
            "INSERT INTO provider_secrets(provider_id, secret, created_at, updated_at)
             VALUES (?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(id.to_string())
        .bind(secret)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(Provider {
            id,
            name: name.to_owned(),
            adapter,
            base_url: base_url.to_owned(),
            credential_configured: true,
            enabled: true,
        })
    }

    pub(crate) async fn resolve_provider_secret(
        &self,
        credential_ref: &str,
    ) -> Result<Option<String>, StoreError> {
        let Some(provider_id) = credential_ref.strip_prefix("secret://") else {
            return Ok(None);
        };
        let secret =
            sqlx::query_scalar("SELECT secret FROM provider_secrets WHERE provider_id = ?")
                .bind(provider_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(secret)
    }

    pub(crate) async fn list_providers(
        &self,
        actor_id: PrincipalId,
    ) -> Result<Vec<Provider>, StoreError> {
        self.require_server_admin(actor_id).await?;
        let rows = sqlx::query(
            "SELECT id, name, adapter, base_url, credential_ref, enabled
             FROM providers ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(provider_from_row).collect()
    }

    pub(crate) async fn get_provider_connection(
        &self,
        actor_id: PrincipalId,
        provider_id: ProviderId,
    ) -> Result<(String, String), StoreError> {
        self.require_server_admin(actor_id).await?;
        sqlx::query_as(
            "SELECT base_url, credential_ref FROM providers
             WHERE id = ? AND enabled = 1",
        )
        .bind(provider_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)
    }

    pub(crate) async fn insert_model(
        &self,
        actor_id: PrincipalId,
        input: &ModelInput,
    ) -> Result<Model, StoreError> {
        self.require_server_admin(actor_id).await?;
        let provider_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM providers WHERE id = ? AND enabled = 1")
                .bind(input.provider_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        if provider_exists != 1 {
            return Err(StoreError::InvalidReference(
                "provider is missing or disabled",
            ));
        }
        let id = ModelId::new();
        sqlx::query(
            "INSERT INTO models(
                id, provider_id, model_name, display_name, capabilities,
                limits_json, defaults_json, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(id.to_string())
        .bind(input.provider_id.to_string())
        .bind(&input.model_name)
        .bind(&input.display_name)
        .bind(input.capabilities.to_string())
        .bind(input.limits.to_string())
        .bind(input.defaults.to_string())
        .execute(&self.pool)
        .await
        .map_err(super::store::map_constraint)?;
        Ok(Model {
            id,
            provider_id: input.provider_id,
            model_name: input.model_name.clone(),
            display_name: input.display_name.clone(),
            capabilities: input.capabilities.clone(),
            limits: input.limits.clone(),
            defaults: input.defaults.clone(),
            enabled: true,
        })
    }

    pub(crate) async fn list_models(
        &self,
        actor_id: PrincipalId,
    ) -> Result<Vec<Model>, StoreError> {
        self.require_server_admin(actor_id).await?;
        let rows = sqlx::query(
            "SELECT id, provider_id, model_name, display_name, capabilities,
                    limits_json, defaults_json, enabled
             FROM models ORDER BY display_name",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(model_from_row).collect()
    }

    pub(crate) async fn insert_ai_member(
        &self,
        actor_id: PrincipalId,
        handle: &str,
        display_name: &str,
        identity_prompt: &str,
        default_model_id: ModelId,
    ) -> Result<AiMember, StoreError> {
        self.require_server_admin(actor_id).await?;
        let mut transaction = self.pool.begin().await?;
        let model_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM models WHERE id = ? AND enabled = 1")
                .bind(default_model_id.to_string())
                .fetch_one(&mut *transaction)
                .await?;
        if model_exists != 1 {
            return Err(StoreError::InvalidReference("model is missing or disabled"));
        }
        let member = insert_ai_member_in_transaction(
            &mut transaction,
            actor_id,
            handle,
            display_name,
            identity_prompt,
            default_model_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(member)
    }

    pub(crate) async fn insert_ai_member_for_model(
        &self,
        actor_id: PrincipalId,
        handle: &str,
        display_name: &str,
        identity_prompt: &str,
        provider_id: ProviderId,
        model_name: &str,
    ) -> Result<AiMember, StoreError> {
        self.require_server_admin(actor_id).await?;
        let mut transaction = self.pool.begin().await?;
        let provider_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM providers WHERE id = ? AND enabled = 1")
                .bind(provider_id.to_string())
                .fetch_one(&mut *transaction)
                .await?;
        if provider_exists != 1 {
            return Err(StoreError::InvalidReference(
                "provider is missing or disabled",
            ));
        }
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM models
             WHERE provider_id = ? AND model_name = ? AND enabled = 1",
        )
        .bind(provider_id.to_string())
        .bind(model_name)
        .fetch_optional(&mut *transaction)
        .await?;
        let model_id = if let Some(existing) = existing {
            ModelId::from_str(&existing)
                .map_err(|_| StoreError::CorruptData("model identifier is invalid"))?
        } else {
            let id = ModelId::new();
            sqlx::query(
                "INSERT INTO models(
                    id, provider_id, model_name, display_name, capabilities,
                    limits_json, defaults_json, created_at, updated_at
                 ) VALUES (?, ?, ?, ?, '{}', '{}', '{}',
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            )
            .bind(id.to_string())
            .bind(provider_id.to_string())
            .bind(model_name)
            .bind(model_name)
            .execute(&mut *transaction)
            .await
            .map_err(super::store::map_constraint)?;
            id
        };
        let member = insert_ai_member_in_transaction(
            &mut transaction,
            actor_id,
            handle,
            display_name,
            identity_prompt,
            model_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(member)
    }

    pub(crate) async fn list_ai_members(
        &self,
        actor_id: PrincipalId,
    ) -> Result<Vec<AiMember>, StoreError> {
        self.require_server_admin(actor_id).await?;
        let rows = sqlx::query(
            "SELECT principal.id, principal.kind, principal.handle, principal.display_name,
                    prompt.prompt AS identity_prompt,
                    profile.identity_prompt_version, profile.default_model_id
             FROM principals AS principal
             JOIN ai_profiles AS profile ON profile.principal_id = principal.id
             JOIN ai_prompt_versions AS prompt
               ON prompt.ai_principal_id = profile.principal_id
              AND prompt.version = profile.identity_prompt_version
             WHERE principal.active = 1 ORDER BY principal.handle",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(ai_member_from_row).collect()
    }

    pub(crate) async fn put_topic_member(
        &self,
        actor: &Principal,
        topic_id: TopicId,
        target_id: PrincipalId,
        role: MembershipRole,
    ) -> Result<TopicMember, StoreError> {
        let mut transaction = self.pool.begin().await?;
        require_human_writer(&mut transaction, topic_id, actor).await?;
        let target = principal_by_id(&mut transaction, target_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if !matches!(target.kind, PrincipalKind::Human | PrincipalKind::Ai) {
            return Err(StoreError::InvalidReference(
                "only Human and AI principals can join a topic",
            ));
        }
        sqlx::query(
            "INSERT INTO topic_memberships(topic_id, principal_id, role, created_at)
             VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(topic_id, principal_id) DO UPDATE SET role = excluded.role",
        )
        .bind(topic_id.to_string())
        .bind(target_id.to_string())
        .bind(role.as_str())
        .execute(&mut *transaction)
        .await?;
        insert_activity_event(
            &mut transaction,
            topic_id,
            None,
            "membership.updated",
            actor.id,
            "principal",
            &target_id.to_string(),
        )
        .await?;
        transaction.commit().await?;
        Ok(TopicMember {
            principal: target,
            role,
        })
    }

    pub(crate) async fn list_topic_members(
        &self,
        actor_id: PrincipalId,
        topic_id: TopicId,
    ) -> Result<Vec<TopicMember>, StoreError> {
        require_topic_reader(&self.pool, topic_id, actor_id).await?;
        let rows = sqlx::query(
            "SELECT principal.id, principal.kind, principal.handle, principal.display_name,
                    membership.role
             FROM topic_memberships AS membership
             JOIN principals AS principal ON principal.id = membership.principal_id
             WHERE membership.topic_id = ? AND principal.active = 1
             ORDER BY principal.handle",
        )
        .bind(topic_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(topic_member_from_row).collect()
    }

    pub(crate) async fn insert_team(
        &self,
        actor: &Principal,
        topic_id: TopicId,
        handle: &str,
        display_name: &str,
    ) -> Result<Team, StoreError> {
        let id = TeamId::new();
        let mut transaction = self.pool.begin().await?;
        require_human_writer(&mut transaction, topic_id, actor).await?;
        let principal_collision: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM principals WHERE handle = ? COLLATE NOCASE")
                .bind(handle)
                .fetch_one(&mut *transaction)
                .await?;
        if principal_collision != 0 {
            return Err(StoreError::Conflict);
        }
        sqlx::query(
            "INSERT INTO teams(
                id, topic_id, handle, display_name, created_by_principal_id,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(id.to_string())
        .bind(topic_id.to_string())
        .bind(handle)
        .bind(display_name)
        .bind(actor.id.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(super::store::map_constraint)?;
        insert_activity_event(
            &mut transaction,
            topic_id,
            None,
            "team.created",
            actor.id,
            "team",
            &id.to_string(),
        )
        .await?;
        transaction.commit().await?;
        Ok(Team {
            id,
            topic_id,
            handle: handle.to_owned(),
            display_name: display_name.to_owned(),
            members: Vec::new(),
        })
    }

    pub(crate) async fn put_team_member(
        &self,
        actor: &Principal,
        team_id: TeamId,
        target_id: PrincipalId,
    ) -> Result<Team, StoreError> {
        let mut transaction = self.pool.begin().await?;
        let team_topic: Option<String> =
            sqlx::query_scalar("SELECT topic_id FROM teams WHERE id = ?")
                .bind(team_id.to_string())
                .fetch_optional(&mut *transaction)
                .await?;
        let topic_id = team_topic
            .ok_or(StoreError::NotFound)?
            .parse::<TopicId>()
            .map_err(|_| StoreError::CorruptData("topic id"))?;
        require_human_writer(&mut transaction, topic_id, actor).await?;
        let member_kind: Option<String> = sqlx::query_scalar(
            "SELECT principal.kind FROM topic_memberships AS membership
             JOIN principals AS principal ON principal.id = membership.principal_id
             WHERE membership.topic_id = ? AND principal.id = ? AND principal.active = 1",
        )
        .bind(topic_id.to_string())
        .bind(target_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        if !matches!(member_kind.as_deref(), Some("human" | "ai")) {
            return Err(StoreError::InvalidReference(
                "team member must be an active Topic member",
            ));
        }
        sqlx::query(
            "INSERT INTO team_members(team_id, principal_id, added_by_principal_id, created_at)
             VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(team_id, principal_id) DO NOTHING",
        )
        .bind(team_id.to_string())
        .bind(target_id.to_string())
        .bind(actor.id.to_string())
        .execute(&mut *transaction)
        .await?;
        insert_activity_event(
            &mut transaction,
            topic_id,
            None,
            "team.member_added",
            actor.id,
            "principal",
            &target_id.to_string(),
        )
        .await?;
        transaction.commit().await?;
        self.get_team_for(actor.id, team_id).await
    }

    pub(crate) async fn list_teams_for(
        &self,
        actor_id: PrincipalId,
        topic_id: TopicId,
    ) -> Result<Vec<Team>, StoreError> {
        require_topic_reader(&self.pool, topic_id, actor_id).await?;
        let rows = sqlx::query(
            "SELECT team.id, team.topic_id, team.handle, team.display_name,
                    member.principal_id
             FROM teams AS team
             LEFT JOIN team_members AS member ON member.team_id = team.id
             WHERE team.topic_id = ? ORDER BY team.handle, member.created_at",
        )
        .bind(topic_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        teams_from_rows(&rows)
    }

    async fn get_team_for(
        &self,
        actor_id: PrincipalId,
        team_id: TeamId,
    ) -> Result<Team, StoreError> {
        let topic: Option<String> = sqlx::query_scalar("SELECT topic_id FROM teams WHERE id = ?")
            .bind(team_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let topic_id = topic
            .ok_or(StoreError::NotFound)?
            .parse::<TopicId>()
            .map_err(|_| StoreError::CorruptData("topic id"))?;
        let teams = self.list_teams_for(actor_id, topic_id).await?;
        teams
            .into_iter()
            .find(|team| team.id == team_id)
            .ok_or(StoreError::NotFound)
    }
}

async fn insert_ai_member_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    actor_id: PrincipalId,
    handle: &str,
    display_name: &str,
    identity_prompt: &str,
    default_model_id: ModelId,
) -> Result<AiMember, StoreError> {
    let id = PrincipalId::new();
    let team_collision: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM teams WHERE handle = ? COLLATE NOCASE")
            .bind(handle)
            .fetch_one(&mut **transaction)
            .await?;
    if team_collision != 0 {
        return Err(StoreError::Conflict);
    }
    sqlx::query(
        "INSERT INTO principals(id, kind, handle, display_name, created_at)
         VALUES (?, 'ai', ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id.to_string())
    .bind(handle)
    .bind(display_name)
    .execute(&mut **transaction)
    .await
    .map_err(super::store::map_constraint)?;
    sqlx::query(
        "INSERT INTO ai_profiles(principal_id, identity_prompt_version, default_model_id)
         VALUES (?, 1, ?)",
    )
    .bind(id.to_string())
    .bind(default_model_id.to_string())
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO ai_prompt_versions(
            ai_principal_id, version, prompt, created_by_principal_id, created_at
         ) VALUES (?, 1, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id.to_string())
    .bind(identity_prompt)
    .bind(actor_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(AiMember {
        principal: Principal {
            id,
            kind: PrincipalKind::Ai,
            handle: handle.to_owned(),
            display_name: display_name.to_owned(),
        },
        identity_prompt: identity_prompt.to_owned(),
        identity_prompt_version: 1,
        default_model_id,
    })
}

async fn require_human_writer(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    topic_id: TopicId,
    actor: &Principal,
) -> Result<(), StoreError> {
    if actor.kind != PrincipalKind::Human {
        return Err(StoreError::PermissionDenied);
    }
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM topic_memberships WHERE topic_id = ? AND principal_id = ?",
    )
    .bind(topic_id.to_string())
    .bind(actor.id.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    if role.as_deref() == Some("write") {
        Ok(())
    } else {
        Err(StoreError::PermissionDenied)
    }
}

async fn require_topic_reader(
    pool: &sqlx::SqlitePool,
    topic_id: TopicId,
    actor_id: PrincipalId,
) -> Result<(), StoreError> {
    let found: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM topic_memberships WHERE topic_id = ? AND principal_id = ?",
    )
    .bind(topic_id.to_string())
    .bind(actor_id.to_string())
    .fetch_one(pool)
    .await?;
    if found == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound)
    }
}

async fn principal_by_id(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: PrincipalId,
) -> Result<Option<Principal>, StoreError> {
    let row = sqlx::query(
        "SELECT id, kind, handle, display_name FROM principals WHERE id = ? AND active = 1",
    )
    .bind(id.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    row.as_ref().map(principal_from_row).transpose()
}

fn principal_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Principal, StoreError> {
    let kind: String = row.try_get("kind")?;
    Ok(Principal {
        id: parse_id(row, "id", "principal id")?,
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

fn provider_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Provider, StoreError> {
    let adapter: String = row.try_get("adapter")?;
    let credential_ref: String = row.try_get("credential_ref")?;
    Ok(Provider {
        id: parse_id(row, "id", "provider id")?,
        name: row.try_get("name")?,
        adapter: ProviderAdapter::from_stored(&adapter)
            .ok_or(StoreError::CorruptData("provider adapter"))?,
        base_url: row.try_get("base_url")?,
        credential_configured: !credential_ref.is_empty(),
        enabled: row.try_get::<i64, _>("enabled")? != 0,
    })
}

fn model_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Model, StoreError> {
    Ok(Model {
        id: parse_id(row, "id", "model id")?,
        provider_id: parse_id(row, "provider_id", "provider id")?,
        model_name: row.try_get("model_name")?,
        display_name: row.try_get("display_name")?,
        capabilities: parse_json(row, "capabilities")?,
        limits: parse_json(row, "limits_json")?,
        defaults: parse_json(row, "defaults_json")?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
    })
}

fn ai_member_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<AiMember, StoreError> {
    Ok(AiMember {
        principal: principal_from_row(row)?,
        identity_prompt: row.try_get("identity_prompt")?,
        identity_prompt_version: row.try_get("identity_prompt_version")?,
        default_model_id: parse_id(row, "default_model_id", "model id")?,
    })
}

fn topic_member_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<TopicMember, StoreError> {
    let role: String = row.try_get("role")?;
    Ok(TopicMember {
        principal: principal_from_row(row)?,
        role: MembershipRole::from_stored(&role)
            .ok_or(StoreError::CorruptData("membership role"))?,
    })
}

fn teams_from_rows(rows: &[sqlx::sqlite::SqliteRow]) -> Result<Vec<Team>, StoreError> {
    let mut teams: BTreeMap<String, Team> = BTreeMap::new();
    for row in rows {
        let id: TeamId = parse_id(row, "id", "team id")?;
        let team = teams.entry(id.to_string()).or_insert(Team {
            id,
            topic_id: parse_id(row, "topic_id", "topic id")?,
            handle: row.try_get("handle")?,
            display_name: row.try_get("display_name")?,
            members: Vec::new(),
        });
        let member: Option<String> = row.try_get("principal_id")?;
        if let Some(member) = member {
            team.members.push(
                PrincipalId::from_str(&member)
                    .map_err(|_| StoreError::CorruptData("team member id"))?,
            );
        }
    }
    Ok(teams.into_values().collect())
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

fn parse_json(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
) -> Result<serde_json::Value, StoreError> {
    let raw: String = row.try_get(column)?;
    serde_json::from_str(&raw).map_err(|_| StoreError::CorruptData("json configuration"))
}
