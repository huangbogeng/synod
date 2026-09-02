use crate::{
    domain::{
        Comment, CommentId, CommentKind, CreateComment, CreateIssue, Issue, IssueType, Principal,
        TopicId, TopicItemId,
    },
    persistence::{Database, StoredCreation},
};

use super::ServiceError;

#[derive(Debug, Clone)]
pub struct IssueService {
    database: Database,
}

impl IssueService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn list_types(&self) -> Result<Vec<IssueType>, ServiceError> {
        self.database.list_issue_types().await.map_err(Into::into)
    }

    pub async fn create_issue(
        &self,
        actor: &Principal,
        topic_id: TopicId,
        issue_type: String,
        title: String,
        body: String,
        parent_issue_id: Option<TopicItemId>,
    ) -> Result<StoredCreation<Issue>, ServiceError> {
        let input = CreateIssue::new(issue_type, title, body, parent_issue_id)?;
        self.database
            .insert_issue(actor, topic_id, &input)
            .await
            .map_err(Into::into)
    }

    pub async fn list_issues(
        &self,
        actor: &Principal,
        topic_id: TopicId,
    ) -> Result<Vec<Issue>, ServiceError> {
        self.database
            .list_issues_for(actor.id, topic_id)
            .await
            .map_err(Into::into)
    }

    pub async fn get_issue(
        &self,
        actor: &Principal,
        issue_id: TopicItemId,
    ) -> Result<Issue, ServiceError> {
        self.database
            .get_issue_for(actor.id, issue_id)
            .await
            .map_err(Into::into)
    }

    pub async fn create_comment(
        &self,
        actor: &Principal,
        issue_id: TopicItemId,
        kind: CommentKind,
        body: String,
        reply_to_comment_id: Option<CommentId>,
    ) -> Result<StoredCreation<Comment>, ServiceError> {
        let input = CreateComment::new(kind, body, reply_to_comment_id)?;
        self.database
            .insert_issue_comment(actor, issue_id, &input)
            .await
            .map_err(Into::into)
    }

    pub async fn list_comments(
        &self,
        actor: &Principal,
        issue_id: TopicItemId,
    ) -> Result<Vec<Comment>, ServiceError> {
        self.database
            .list_issue_comments_for(actor.id, issue_id)
            .await
            .map_err(Into::into)
    }
}
