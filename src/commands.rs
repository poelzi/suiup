use clap::{Parser, Subcommand, ValueEnum};

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
    #[command(about = "Install one or more components")]
    Install {
        #[arg(
            num_args = 1..=2,
            help = "Component to install with optional version (e.g. 'sui', 'sui testnet-v1.39.3', 'sui testnet')"
        )]
        components: Vec<String>,
        #[arg(
            long,
            required = false,
            value_name = "branch",
            default_missing_value = "main",
            num_args = 0..=1,
            help = "Install from a branch. If none provided, main is used. Note that this requires Rust & cargo to be installed."
        )]
        nightly: Option<String>,
        #[arg(short, long, help = "Accept defaults without prompting")]
        yes: bool,
    },
    #[command(about = "Show installed and active Sui binaries")]
    Show,
    #[command(about = "Update binary")]
    Update { name: String },
    #[command(about = "Show the path where default binaries are installed")]
    Which,
    #[command(about = "Generate shell completion scripts")]
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
pub(crate) enum ComponentCommands {
    #[command(about = "List available components")]
    List,
    #[command(about = "Add one or more components")]
    Add {
        #[arg(
            num_args = 1..=2,
            help = "Component to install with optional version (e.g. 'sui', 'sui testnet-v1.39.3', 'sui testnet')"
        )]
        components: Vec<String>,
        #[arg(
            long,
            help = "Whether to install the debug version of the component (only available for sui). Default is false."
        )]
        debug: bool,
        #[arg(
            long,
            required = false,
            value_name = "branch",
            default_missing_value = "main",
            num_args = 0..=1,
            help = "Install from a branch. If none provided, main is used. Note that this requires Rust & cargo to be installed."
        )]
        nightly: Option<String>,
        #[arg(short, long, help = "Accept defaults without prompting")]
        yes: bool,
    },
    #[command(
        about = "Remove one or more components. By default, the binary from each release will be removed."
    )]
    Remove {
        #[arg(value_enum)]
        binaries: Vec<SuiComponent>,
    },
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
        #[arg(long, help = "Version of the component to set to default.")]
        version: Option<String>,
        #[arg(
            long,
            help = "Whether to set the debug version of the component as default (only available for sui)."
        )]
        debug: bool,
    },
}

#[derive(Clone, Debug, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum SuiComponent {
    #[value(name = "sui")]
    Sui,
    #[value(name = "sui-bridge")]
    SuiBridge,
    #[value(name = "sui-faucet")]
    SuiFaucet,
    #[value(name = "walrus")]
    Walrus,
    #[value(name = "mvr")]
    Mvr,
}

impl std::fmt::Display for SuiComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuiComponent::Sui => write!(f, "sui"),
            SuiComponent::SuiBridge => write!(f, "sui-bridge"),
            SuiComponent::SuiFaucet => write!(f, "sui-faucet"),
            SuiComponent::Walrus => write!(f, "walrus"),
            SuiComponent::Mvr => write!(f, "mvr"),
        }
    }
}

pub fn parse_component_with_version(s: &str) -> Result<(SuiComponent, Option<String>), String> {
    let parts: Vec<&str> = s.split_whitespace().collect();

    match parts.len() {
        1 => {
            let component = SuiComponent::from_str(parts[0], true)
                .map_err(|_| format!("Invalid component name: {}", parts[0]))?;
            Ok((component, None))
        }
        2 => {
            let component = SuiComponent::from_str(parts[0], true)
                .map_err(|_| format!("Invalid component name: {}", parts[0]))?;
            Ok((component, Some(parts[1].to_string())))
        }
        _ => Err("Invalid format. Use 'component' or 'component version'".to_string()),
    }
}

impl std::str::FromStr for SuiComponent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sui" => Ok(SuiComponent::Sui),
            "sui-bridge" => Ok(SuiComponent::SuiBridge),
            "sui-faucet" => Ok(SuiComponent::SuiFaucet),
            "walrus" => Ok(SuiComponent::Walrus),
            "mvr" => Ok(SuiComponent::Mvr),
            _ => Err(format!("Unknown component: {}", s)),
        }
    }
}
