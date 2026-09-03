use std::{path::Path, str::FromStr, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

mod dispatches;
mod issues;
mod members;
mod store;

pub use issues::StoredCreation;
pub use store::{BootstrapOutput, StoreError};

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
            "ai_profiles",
            "comments",
            "conversations",
            "dispatch_mentions",
            "dispatch_target_sources",
            "dispatch_targets",
            "dispatches",
            "issue_types",
            "issues",
            "jobs",
            "models",
            "notifications",
            "principal_tokens",
            "principals",
            "proposals",
            "providers",
            "runs",
            "team_members",
            "teams",
            "topic_items",
            "topics",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "missing {expected}"
            );
        }
    }

    #[tokio::test]
    async fn bootstrap_token_is_hashed_and_bootstrap_is_single_use() {
        let database = Database::connect(Path::new(":memory:")).await.unwrap();
        let bootstrap = database
            .bootstrap_human("alice", "Alice", "test")
            .await
            .unwrap();

        let authenticated = database
            .authenticate(&bootstrap.token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(authenticated.id, bootstrap.principal.id);

        let plaintext_matches: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM principal_tokens WHERE CAST(token_hash AS TEXT) = ?",
        )
        .bind(&bootstrap.token)
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(plaintext_matches, 0);

        let Err(repeated) = database.bootstrap_human("bob", "Bob", "test").await else {
            panic!("second bootstrap unexpectedly succeeded");
        };
        assert!(matches!(repeated, StoreError::AlreadyBootstrapped));
    }
}
