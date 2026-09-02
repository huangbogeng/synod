use crate::{
    domain::{
        MembershipRole, Principal, PrincipalId, Team, TeamId, TopicId, TopicMember, validate_handle,
    },
    persistence::Database,
};

use super::ServiceError;

#[derive(Debug, Clone)]
pub struct MembershipService {
    database: Database,
}

impl MembershipService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn put_topic_member(
        &self,
        actor: &Principal,
        topic_id: TopicId,
        principal_id: PrincipalId,
        role: MembershipRole,
    ) -> Result<TopicMember, ServiceError> {
        self.database
            .put_topic_member(actor, topic_id, principal_id, role)
            .await
            .map_err(Into::into)
    }

    pub async fn list_topic_members(
        &self,
        actor: &Principal,
        topic_id: TopicId,
    ) -> Result<Vec<TopicMember>, ServiceError> {
        self.database
            .list_topic_members(actor.id, topic_id)
            .await
            .map_err(Into::into)
    }

    pub async fn create_team(
        &self,
        actor: &Principal,
        topic_id: TopicId,
        handle: String,
        display_name: String,
    ) -> Result<Team, ServiceError> {
        validate_handle(&handle)?;
        if display_name.trim().is_empty() || display_name.chars().count() > 100 {
            return Err(ServiceError::InvalidReference(
                "team display name is invalid",
            ));
        }
        self.database
            .insert_team(actor, topic_id, &handle, display_name.trim())
            .await
            .map_err(Into::into)
    }

    pub async fn list_teams(
        &self,
        actor: &Principal,
        topic_id: TopicId,
    ) -> Result<Vec<Team>, ServiceError> {
        self.database
            .list_teams_for(actor.id, topic_id)
            .await
            .map_err(Into::into)
    }

    pub async fn put_team_member(
        &self,
        actor: &Principal,
        team_id: TeamId,
        principal_id: PrincipalId,
    ) -> Result<Team, ServiceError> {
        self.database
            .put_team_member(actor, team_id, principal_id)
            .await
            .map_err(Into::into)
    }
}
