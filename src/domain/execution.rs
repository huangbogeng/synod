use serde::{Deserialize, Serialize};

use super::{CommentId, ContextSnapshotId, PrincipalId, RunId, TopicId, TopicItemId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextTopic {
    pub id: TopicId,
    pub title: String,
    pub description: String,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextIssue {
    pub id: TopicItemId,
    pub title: String,
    pub issue_type: String,
    pub state: String,
    pub body: String,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextTrigger {
    pub source_type: String,
    pub source_id: String,
    pub source_revision: i64,
    pub author_handle: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextComment {
    pub id: CommentId,
    pub author_id: PrincipalId,
    pub author_handle: String,
    pub author_kind: String,
    pub kind: String,
    pub body: String,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextInput {
    pub topic: ContextTopic,
    pub issue: ContextIssue,
    pub trigger: ContextTrigger,
    pub timeline: Vec<ContextComment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSource {
    pub source_type: String,
    pub source_id: String,
    pub revision: i64,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextManifest {
    pub run_id: RunId,
    pub sources: Vec<ContextSource>,
    pub omissions: Vec<String>,
    pub estimated_input_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub id: ContextSnapshotId,
    pub run_id: RunId,
    pub manifest: ContextManifest,
    pub input: ContextInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub run_id: RunId,
    pub context_snapshot_id: ContextSnapshotId,
    pub system_prompt: String,
    pub context: ContextInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResponse {
    pub text: String,
    #[serde(default)]
    pub usage: serde_json::Value,
    pub provider_request_id: Option<String>,
}
