// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Error;
use clap::CommandFactory;
use clap::Parser;

use handle_commands::initialize;
use handle_commands::{handle_component, handle_default, handle_show, handle_update, handle_which};

mod commands;
mod handle_commands;
mod mvr;
mod paths;
mod types;
mod walrus;
use commands::{Commands, ComponentCommands, Suiup};

const GITHUB_REPO: &str = "MystenLabs/sui";
const RELEASES_ARCHIVES_FOLDER: &str = "releases";

#[tokio::main]
async fn main() -> Result<(), Error> {
    initialize()?;
    let args = Suiup::parse();

    match args.command {
        Commands::Default(cmd) => handle_default(cmd)?,
        Commands::Install {
            components,
            nightly,
            debug,
            yes,
        } => {
            handle_component(ComponentCommands::Add {
                components,
                nightly,
                debug,
                yes,
            })
            .await?
        }
        Commands::Remove { binary } => {
            handle_component(ComponentCommands::Remove { binary }).await?
        }
        Commands::List => handle_component(ComponentCommands::List).await?,
        Commands::Show => handle_show()?,
        Commands::Update { name, yes } => handle_update(name, yes).await?,
        Commands::Which => handle_which()?,
        Commands::Completion { shell } => {
            let mut cmd = Suiup::command();
            // Generate to string first to validate the output
            let mut buf = Vec::new();
            clap_complete::generate(shell, &mut cmd, "suiup", &mut buf);

            // Print to stdout if generation was successful
            if let Ok(s) = String::from_utf8(buf) {
                print!("{}", s);
            }
            // print_completion_instructions(&shell);
        }
    }
    Ok(())
}
