use clap::Parser;
use handle_commands::initialize;
use handle_commands::{
    handle_component, handle_default, handle_override, handle_show, handle_update, handle_which,
};

use anyhow::anyhow;
use anyhow::Error;

mod commands;
mod handle_commands;
mod types;
use commands::{Commands, Suiup};

const GITHUB_REPO: &str = "mystenlabs/sui";
const RELEASES_ARCHIVES_FOLDER: &str = "releases";
const SUIUP_FOLDER: &str = ".suiup";
const BINARIES_FOLDER: &str = "binaries";
const DEFAULT_BIN_FOLDER: &str = "default-bin";
const DEFAULT_VERSION_FILENAME: &str = "default_version.json";
const INSTALLED_BINARIES_FILENAME: &str = "installed_binaries.json";

#[tokio::main]
async fn main() -> Result<(), Error> {
    initialize()?;
    let args = Suiup::parse();

    match args.command {
        Commands::Component(cmd) => handle_component(cmd).await.map_err(|e| anyhow!("{e}"))?,
        Commands::Default(cmd) => handle_default(cmd)?,
        Commands::Show => handle_show()?,
        Commands::Update { name } => handle_update(name).await?,
        Commands::Override => handle_override(),
        Commands::Which => handle_which()?,
    }
    Ok(())
}
