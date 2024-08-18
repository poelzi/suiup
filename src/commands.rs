use clap::{Parser, Subcommand};

use crate::types::Network;

#[derive(Parser)]
#[command(name = "suiup")]
#[command(about = "Sui CLI version manager.")]
pub(crate) struct Suiup {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    #[command(subcommand, about = "List, add, or remove components")]
    Component(ComponentCommands),
    #[command(subcommand, about = "Get or set the default Sui components' version")]
    Default(DefaultCommands),
    #[command(about = "Show installed and active Sui binaries")]
    Show,
    #[command(about = "Update binary")]
    Update,
    #[command(about = "Override project’s CLI version")]
    Override,
    #[command(about = "Show the path of the active CLI binary")]
    Which,
}

#[derive(Subcommand)]
pub(crate) enum ComponentCommands {
    #[command(about = "List available components")]
    List,
    #[command(about = "Add one or more components")]
    Add {
        name: Vec<String>,
        #[arg(long, value_enum, default_value_t = Network::Testnet)]
        network: Network,
        #[arg(
            long,
            help = "Version of the component to install. If not provided, the latest version will be installed."
        )]
        version: Option<String>,
    },
    #[command(
        about = "Remove one or more components. By default, the binary from each release will be removed. Use --network and --version to remove a specific version from a specific release."
    )]
    Remove { name: Vec<String> },
}

#[derive(Subcommand)]
pub(crate) enum DefaultCommands {
    #[command(about = "Get the default Sui CLI version")]
    Get,
    #[command(about = "Set the default Sui CLI version")]
    Set {
        // #[arg(
        // long,
        // help = "Component(s) to be set as default. Must be provided, together with the network. If no version is provided, the latest version available locally will be set."
        // )]
        /// Component(s) to be set as default. Must be provided, together with the network. If no
        /// version is provided, the latest version available locally will be set.
        name: Vec<String>,
        #[arg(short, long, value_enum, default_value_t = Network::Testnet)]
        network: Network,
        #[arg(short, long, help = "Version of the component to set to default.")]
        /// Version of the component to set to default.
        version: String,
    },
}
