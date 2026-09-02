use std::time::Duration;

use tokio::time;

use crate::persistence::Database;

pub async fn check_once(database: &Database) -> Result<(), sqlx::Error> {
    database.healthcheck().await
}

pub async fn run(database: Database) -> Result<(), sqlx::Error> {
    tracing::info!("worker started");
    let mut interval = time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => check_once(&database).await?,
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
