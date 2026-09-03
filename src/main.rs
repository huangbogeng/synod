use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand, ValueEnum};
use synod::{
    api::{self, AppState},
    config::{ServerArgs, StorageArgs},
    persistence::Database,
    services::{IdentityService, MaintenanceService},
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
    /// Perform explicit local configuration through Synod's service boundary.
    Config {
        #[command(flatten)]
        storage: StorageArgs,
        #[command(subcommand)]
        action: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Delete every Topic and its discussion data. Providers and Members remain.
    ClearTopics {
        /// Confirm the irreversible deletion after reviewing the target database.
        #[arg(long)]
        confirm: bool,
    },
    /// Create or update one AI Member by handle.
    SetMember {
        #[arg(long)]
        handle: String,
        #[arg(long)]
        display_name: String,
        /// Existing Provider display name, matched case-insensitively.
        #[arg(long)]
        provider: String,
        /// Exact model identifier sent to the Provider.
        #[arg(long)]
        model: String,
        /// Read the identity Prompt from this UTF-8 file.
        #[arg(long)]
        prompt_file: PathBuf,
        /// Member-level sampling temperature, from 0 through 2.
        #[arg(long, value_parser = parse_temperature)]
        temperature: Option<f64>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        output: OutputFormat,
    },
    /// List active AI Members and their execution defaults.
    ListMembers {
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        output: OutputFormat,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
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
        Command::Config { storage, action } => {
            let database = Database::connect(&storage.database).await?;
            let service = MaintenanceService::new(database);
            match action {
                ConfigCommand::ClearTopics { confirm } => {
                    if !confirm {
                        println!(
                            "{}",
                            serde_json::json!({
                                "would_delete_topics": service.count_topics().await?,
                                "deleted_topics": 0,
                                "confirm_with": "--confirm"
                            })
                        );
                        return Ok(());
                    }
                    let deleted_topics = service.clear_all_topics().await?;
                    println!("{}", serde_json::json!({"deleted_topics": deleted_topics}));
                }
                ConfigCommand::SetMember {
                    handle,
                    display_name,
                    provider,
                    model,
                    prompt_file,
                    temperature,
                    output,
                } => {
                    let identity_prompt = std::fs::read_to_string(&prompt_file)?;
                    let mut execution_defaults = serde_json::Map::new();
                    if let Some(temperature) = temperature {
                        execution_defaults
                            .insert("temperature".to_owned(), serde_json::json!(temperature));
                    }
                    let member = service
                        .configure_ai_member(
                            handle,
                            display_name,
                            identity_prompt,
                            provider,
                            model,
                            serde_json::Value::Object(execution_defaults),
                        )
                        .await?;
                    print_members(&[member], output)?;
                }
                ConfigCommand::ListMembers { output } => {
                    print_members(&service.list_ai_members().await?, output)?;
                }
            }
            Ok(())
        }
    }
}

fn parse_temperature(value: &str) -> Result<f64, String> {
    let temperature = value
        .parse::<f64>()
        .map_err(|_| "temperature must be a number from 0 through 2".to_owned())?;
    if temperature.is_finite() && (0.0..=2.0).contains(&temperature) {
        Ok(temperature)
    } else {
        Err("temperature must be a finite number from 0 through 2".to_owned())
    }
}

fn print_members(
    members: &[synod::domain::AiMember],
    output: OutputFormat,
) -> Result<(), serde_json::Error> {
    match output {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(members)?),
        OutputFormat::Text => {
            for member in members {
                println!(
                    "@{}\t{}\t{}",
                    member.principal.handle,
                    member.principal.display_name,
                    member.execution_defaults
                );
            }
        }
    }
    Ok(())
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
