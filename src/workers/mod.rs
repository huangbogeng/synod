use std::time::Duration;

use tokio::time;

use crate::persistence::{Database, StoreError};
use crate::{
    providers::{HttpGateway, ModelGateway, ProviderError},
    services::{ExecutionService, ServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerPass {
    pub resolved_dispatch: bool,
    pub executed_run: bool,
}

pub async fn process_once(database: &Database) -> Result<bool, StoreError> {
    database.healthcheck().await?;
    database.resolve_next_dispatch().await
}

pub async fn execute_once_with<G>(
    database: &Database,
    gateway: &G,
) -> Result<bool, crate::services::ServiceError>
where
    G: ModelGateway,
{
    ExecutionService::new(database.clone())
        .execute_once(gateway)
        .await
}

pub async fn run_once(database: &Database) -> Result<WorkerPass, WorkerError> {
    let gateway = HttpGateway::new(database.clone())?;
    run_pass(database, &gateway).await
}

async fn run_pass<G>(database: &Database, gateway: &G) -> Result<WorkerPass, WorkerError>
where
    G: ModelGateway,
{
    let resolved_dispatch = process_once(database).await?;
    let executed_run = execute_once_with(database, gateway).await?;
    Ok(WorkerPass {
        resolved_dispatch,
        executed_run,
    })
}

pub async fn run(database: Database) -> Result<(), WorkerError> {
    tracing::info!("worker started");
    let gateway = HttpGateway::new(database.clone())?;
    let mut interval = time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let pass = run_pass(&database, &gateway).await?;
                if pass.resolved_dispatch {
                    tracing::info!("resolved pending dispatch");
                }
                if pass.executed_run {
                    tracing::info!("executed queued run");
                }
            },
            signal = tokio::signal::ctrl_c() => {
                if let Err(error) = signal {
                    tracing::warn!(%error, "failed to listen for shutdown signal");
                }
                tracing::info!("worker stopped");
                return Ok(());
            }
        }
    }
}
