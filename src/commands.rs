use clap::{Parser, Subcommand};

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
        #[arg(long, required=false, default_missing_value = "testnet", num_args=0..=1)]
        network_release: Option<String>,
        #[arg(
            long,
            help = "Version of the component to install. If not provided, the latest version will be installed."
        )]
        version: Option<String>,
        #[arg(
            long,
            required = false,
            value_name = "branch",
            conflicts_with_all = &["version", "network_release"],
            default_missing_value = "main",
            num_args = 0..=1,
            help = "Install from a branch. If none provided, main is used. Note that this requires Rust & cargo to be installed."
        )]
        nightly: Option<String>,
    },
    #[command(about = "Show installed and active Sui binaries")]
    Show,
    #[command(about = "Update binary")]
    Update { name: String },
    #[command(about = "Show the path where default binaries are installed")]
    Which,
}

#[derive(Subcommand)]
pub(crate) enum ComponentCommands {
    #[command(about = "List available components")]
    List,
    #[command(about = "Add one or more components")]
    Add {
        name: Vec<String>,
        #[arg(long, default_missing_value = "testnet", required = false, num_args=0..=1)]
        network_release: String,
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
        #[arg(
            long,
            required = false,
            value_name = "branch",
            conflicts_with_all = &["version", "network_release"],
            default_missing_value = "main",
            num_args = 0..=1,
            help = "Install from a branch. If none provided, main is used. Note that this requires Rust & cargo to be installed."
        )]
        nightly: Option<String>,
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
        /// Component to be set as default
        name: String,
        #[arg(long, required=false, default_missing_value = "testnet", num_args=0..=1)]
        network_release: Option<String>,
        #[arg(
            long,
            help = "Version of the component to set to default."
        )]
        version: Option<String>,
        #[arg(
            long,
            help = "Whether to set the debug version of the component as default (only available for sui)."
        )]
        debug: bool,
    },
}
