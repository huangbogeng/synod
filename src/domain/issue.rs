use serde::{Deserialize, Serialize};

use super::{CommentId, PrincipalId, TopicId, TopicItemId, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueType {
    pub key: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub id: TopicItemId,
    pub topic_id: TopicId,
    pub number: i64,
    pub issue_type: String,
    pub state: super::IssueState,
    pub title: String,
    pub body: String,
    pub parent_issue_id: Option<TopicItemId>,
    pub author_id: PrincipalId,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateIssue {
    pub issue_type: String,
    pub title: String,
    pub body: String,
    pub parent_issue_id: Option<TopicItemId>,
}

impl CreateIssue {
    pub fn new(
        issue_type: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        parent_issue_id: Option<TopicItemId>,
    ) -> Result<Self, ValidationError> {
        let issue_type = issue_type.into();
        let title = title.into().trim().to_owned();
        let body = body.into();
        if !valid_slug(&issue_type, 32) {
            return Err(ValidationError::InvalidIssueType);
        }
        if title.is_empty() || title.chars().count() > 200 {
            return Err(ValidationError::InvalidTitle);
        }
        if body.chars().count() > 100_000 {
            return Err(ValidationError::BodyTooLong);
        }
        Ok(Self {
            issue_type,
            title,
            body,
            parent_issue_id,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentKind {
    Discussion,
    Direction,
    Evidence,
    Progress,
    Result,
}

impl CommentKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discussion => "discussion",
            Self::Direction => "direction",
            Self::Evidence => "evidence",
            Self::Progress => "progress",
            Self::Result => "result",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub item_id: TopicItemId,
    pub author_id: PrincipalId,
    pub kind: CommentKind,
    pub body: String,
    pub revision: i64,
    pub reply_to_comment_id: Option<CommentId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateComment {
    pub kind: CommentKind,
    pub body: String,
    pub reply_to_comment_id: Option<CommentId>,
}

impl CreateComment {
    pub fn new(
        kind: CommentKind,
        body: impl Into<String>,
        reply_to_comment_id: Option<CommentId>,
    ) -> Result<Self, ValidationError> {
        let body = body.into();
        if body.trim().is_empty() {
            return Err(ValidationError::EmptyBody);
        }
        if body.chars().count() > 100_000 {
            return Err(ValidationError::BodyTooLong);
        }
        Ok(Self {
            kind,
            body,
            reply_to_comment_id,
        })
    }
}

fn valid_slug(value: &str, max_len: usize) -> bool {
    (2..=max_len).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_comment_inputs_are_bounded() {
        assert!(CreateIssue::new("research", "Question", "", None).is_ok());
        assert!(CreateIssue::new("Research", "Question", "", None).is_err());
        assert!(CreateComment::new(CommentKind::Discussion, "  ", None).is_err());
    }
}
