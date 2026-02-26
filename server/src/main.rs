mod cmd;
mod conf;
mod internal;
mod pkg;
mod prelude;
mod state;

use clap::Parser;
use crate::prelude::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cmd::Cli::parse();
    cli.run().await?;
    Ok(())
}
