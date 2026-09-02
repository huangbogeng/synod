use std::process::ExitCode;

use clap::{Parser, Subcommand};
use synod::{
    api::{self, AppState},
    config::RuntimeArgs,
    persistence::Database,
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
    Server(RuntimeArgs),
    /// Run the durable job worker.
    Worker {
        #[command(flatten)]
        runtime: RuntimeArgs,
        /// Check storage and exit without entering the worker loop.
        #[arg(long)]
        once: bool,
    },
    /// Run the local single-process development profile.
    Dev(RuntimeArgs),
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
        Command::Worker { runtime, once } => {
            let database = Database::connect(&runtime.database).await?;
            if once {
                workers::check_once(&database).await?;
                tracing::info!("worker storage check completed");
                Ok(())
            } else {
                workers::run(database).await?;
                Ok(())
            }
        }
        Command::Dev(runtime) => {
            let worker_database = Database::connect(&runtime.database).await?;
            let worker = tokio::spawn(workers::run(worker_database));
            tracing::info!("development profile started server and worker");

            let result = serve(runtime).await;
            worker.abort();
            let _ = worker.await;
            result
        }
    }
}

async fn serve(runtime: RuntimeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::connect(&runtime.database).await?;
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
