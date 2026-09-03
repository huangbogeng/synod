use crate::{
    domain::{
        ContextSnapshot, ContextSnapshotId, Dispatch, DispatchId, Notification, Principal, Run,
        RunId, TopicId,
    },
    persistence::Database,
};

use super::ServiceError;

#[derive(Debug, Clone)]
pub struct DispatchService {
    database: Database,
}

impl DispatchService {
    #[must_use]
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn get_dispatch(
        &self,
        actor: &Principal,
        dispatch_id: DispatchId,
    ) -> Result<Dispatch, ServiceError> {
        self.database
            .get_dispatch_for(actor.id, dispatch_id)
            .await
            .map_err(Into::into)
    }

    pub async fn get_run(&self, actor: &Principal, run_id: RunId) -> Result<Run, ServiceError> {
        self.database
            .get_run_for(actor.id, run_id)
            .await
            .map_err(Into::into)
    }

    pub async fn list_runs(
        &self,
        actor: &Principal,
        topic_id: TopicId,
    ) -> Result<Vec<Run>, ServiceError> {
        self.database
            .list_runs_for(actor.id, topic_id)
            .await
            .map_err(Into::into)
    }

    pub async fn list_notifications(
        &self,
        actor: &Principal,
    ) -> Result<Vec<Notification>, ServiceError> {
        self.database
            .list_notifications_for(actor.id)
            .await
            .map_err(Into::into)
    }

    pub async fn get_context_snapshot(
        &self,
        actor: &Principal,
        snapshot_id: ContextSnapshotId,
    ) -> Result<ContextSnapshot, ServiceError> {
        self.database
            .get_context_snapshot_for(actor.id, snapshot_id)
            .await
            .map_err(Into::into)
    }
}
