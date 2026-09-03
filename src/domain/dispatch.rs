use serde::{Deserialize, Serialize};

use super::{
    ContextSnapshotId, ConversationId, DispatchId, ModelId, NotificationId, PrincipalId,
    PrincipalKind, RunConclusion, RunId, RunStatus, TopicId, TopicItemId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    Pending,
    Dispatched,
    PartiallyDispatched,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTargetOutcome {
    Notified,
    Queued,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionSourceKind {
    Direct,
    Team,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchTargetSource {
    pub mention_handle: String,
    pub source_kind: MentionSourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchTarget {
    pub principal_id: Option<PrincipalId>,
    pub handle: String,
    pub principal_kind: Option<PrincipalKind>,
    pub outcome: DispatchTargetOutcome,
    pub notification_id: Option<NotificationId>,
    pub run_id: Option<RunId>,
    pub skip_reason: Option<String>,
    pub sources: Vec<DispatchTargetSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dispatch {
    pub id: DispatchId,
    pub topic_id: TopicId,
    pub source_type: String,
    pub source_id: String,
    pub source_revision: i64,
    pub author_id: PrincipalId,
    pub status: DispatchStatus,
    pub mentions: Vec<String>,
    pub targets: Vec<DispatchTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub dispatch_id: DispatchId,
    pub topic_id: TopicId,
    pub item_id: TopicItemId,
    pub ai_member_id: PrincipalId,
    pub conversation_id: ConversationId,
    pub identity_prompt_version: i64,
    pub model_id: ModelId,
    pub model_parameters: serde_json::Value,
    pub context_snapshot_id: Option<ContextSnapshotId>,
    pub status: RunStatus,
    pub conclusion: Option<RunConclusion>,
    pub retry_of_run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: NotificationId,
    pub dispatch_id: DispatchId,
    pub recipient_id: PrincipalId,
    pub kind: String,
    pub read: bool,
}
