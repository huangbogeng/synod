use std::{net::SocketAddr, path::PathBuf};

use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct RuntimeArgs {
    /// Address used by the HTTP server.
    #[arg(long, env = "SYNOD_BIND", default_value = "127.0.0.1:3030")]
    pub bind: SocketAddr,

    /// SQLite database path. Parent directories must already exist.
    #[arg(long, env = "SYNOD_DATABASE", default_value = "synod.db")]
    pub database: PathBuf,
}
