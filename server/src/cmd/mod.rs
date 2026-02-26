mod serve;

use clap::{Parser, Subcommand};
use crate::prelude::Result;

#[derive(Debug, Parser)]
#[command(name = "litedocs-server")]
#[command(about = "Litedocs backend service")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Serve(serve::ServeCmd),
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Serve(cmd) => cmd.run().await,
        }
    }
}
