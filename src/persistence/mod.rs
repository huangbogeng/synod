use std::{path::Path, str::FromStr, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(path: &Path) -> Result<Self, sqlx::Error> {
        let options = if path == Path::new(":memory:") {
            SqliteConnectOptions::from_str("sqlite::memory:")?
        } else {
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
        }
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .journal_mode(SqliteJournalMode::Wal);

        let max_connections = if path == Path::new(":memory:") { 1 } else { 5 };
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;

        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn healthcheck(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_create_the_core_schema() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        database.healthcheck().await.unwrap();

        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .fetch_all(database.pool())
                .await
                .unwrap();

        for expected in [
            "activity_events",
            "comments",
            "issues",
            "jobs",
            "principals",
            "proposals",
            "topic_items",
            "topics",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "missing {expected}"
            );
        }
    }
}
