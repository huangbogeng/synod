use std::time::Duration;

use tokio::time;

use crate::persistence::{Database, StoreError};
use crate::{providers::ModelGateway, services::ExecutionService};

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

pub async fn run(database: Database) -> Result<(), StoreError> {
    tracing::info!("worker started");
    let mut interval = time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if process_once(&database).await? {
                    tracing::info!("resolved pending dispatch");
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
