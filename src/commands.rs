use clap::{Parser, Subcommand};

use crate::types::Network;

#[derive(Parser)]
#[command(name = "suiup")]
#[command(about = "Sui Tooling Version Manager.")]
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
    #[command(about = "Install one or more components. Shortcut of `suiup component add`")]
    Install {
        name: Vec<String>,
        #[arg(long, value_enum, default_value_t = Network::Testnet)]
        network_release: Network,
        #[arg(
            long,
            help = "Version of the component to install. If not provided, the latest version will be installed."
        )]
        version: Option<String>,
    },
    #[command(about = "Show installed and active Sui binaries")]
    Show,
    #[command(about = "Update binary")]
    Update { name: String },
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
        network_release: Network,
        #[arg(
            long,
            help = "Version of the component to install. If not provided, the latest version will be installed."
        )]
        version: Option<String>,
        #[arg(
            long,
            help = "Whether to install the debug version of the component (only available for sui). Default is false."
        )]
        debug: bool,
    },
    #[command(
        about = "Remove one or more components. By default, the binary from each release will be removed."
    )]
    Remove { binaries: Vec<String> },
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
        /// Component to be set as default. If no network is provided, testnet will be selected. If no
        /// version is provided, the latest version available locally will be set.
        name: String,
        #[arg(short, long, value_enum, default_value_t = Network::Testnet)]
        network_release: Network,
        #[arg(
            short,
            long,
            help = "Version of the component to set to default.",
            requires = "network_release"
        )]
        /// Version of the component to set to default.
        version: Option<String>,
        #[arg(
            short,
            long,
            help = "Whether to set the debug version of the component as default (only available for sui)."
        )]
        debug: bool,
    },
}
