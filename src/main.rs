use std::process::ExitCode;

use clap::{Parser, Subcommand};
use synod::{
    api::{self, AppState},
    config::{ServerArgs, StorageArgs},
    persistence::Database,
    services::IdentityService,
    workers,
};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "synod", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the HTTP API and Web server.
    Server(ServerArgs),
    /// Run the durable job worker.
    Worker {
        #[command(flatten)]
        storage: StorageArgs,
        /// Resolve at most one Dispatch and execute at most one Run, then exit.
        #[arg(long)]
        once: bool,
    },
    /// Run the local single-process development profile.
    Dev(ServerArgs),
    /// Create the first local Human Member and bearer token.
    Bootstrap {
        #[command(flatten)]
        storage: StorageArgs,
        /// Lowercase handle for the first Human Member.
        #[arg(long, default_value = "admin")]
        handle: String,
        /// Display name for the first Human Member.
        #[arg(long, default_value = "Administrator")]
        display_name: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "synod stopped");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Server(runtime) => serve(runtime).await,
        Command::Worker { storage, once } => {
            let database = Database::connect(&storage.database).await?;
            if once {
                let pass = workers::run_once(&database).await?;
                tracing::info!(
                    resolved_dispatch = pass.resolved_dispatch,
                    executed_run = pass.executed_run,
                    "worker pass completed"
                );
                Ok(())
            } else {
                workers::run(database).await?;
                Ok(())
            }
        }
        Command::Dev(runtime) => {
            let worker_database = Database::connect(&runtime.storage.database).await?;
            let worker = tokio::spawn(workers::run(worker_database));
            tracing::info!("development profile started server and worker");

            let result = serve(runtime).await;
            worker.abort();
            let _ = worker.await;
            result
        }
        Command::Bootstrap {
            storage,
            handle,
            display_name,
        } => {
            let database = Database::connect(&storage.database).await?;
            let bootstrap = IdentityService::new(database)
                .bootstrap_human(&handle, &display_name)
                .await?;
            println!(
                "Human: {} ({})",
                bootstrap.principal.handle, bootstrap.principal.id
            );
            println!("Token: {}", bootstrap.token);
            println!("Store this token securely; Synod cannot display it again.");
            Ok(())
        }
    }
}

async fn serve(runtime: ServerArgs) -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::connect(&runtime.storage.database).await?;
    let listener = tokio::net::TcpListener::bind(runtime.bind).await?;
    tracing::info!(address = %runtime.bind, "server listening");

    axum::serve(listener, api::router(AppState { database }))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!(%error, "failed to listen for shutdown signal");
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("synod=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
