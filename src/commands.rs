use anyhow::{anyhow, bail, Error};
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
        about = "Remove one. By default, the binary from each release will be removed. Use --version to specify which exact version to remove"
    )]
    Remove {
        #[arg(value_enum)]
        binary: BinaryName,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum DefaultCommands {
    #[command(about = "Get the default Sui CLI version")]
    Get,
    #[command(about = "Set the default Sui CLI version")]
    Set {
        #[arg(
            help = "Component to be set as default and the version (e.g. 'sui testnet-v1.39.3', 'sui testnet' -- this will use an installed binary that has the higest testnet version)"
        )]
        name: Vec<String>,
        #[arg(
            long,
            help = "Whether to set the debug version of the component as default (only available for sui)."
        )]
        debug: bool,
    },
}

#[derive(Clone, Debug, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum BinaryName {
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

impl BinaryName {
    pub fn repo_url(&self) -> &str {
        match self {
            BinaryName::Mvr => "https://github.com/MystenLabs/mvr",
            BinaryName::Walrus => "https://github.com/MystenLabs/walrus",
            _ => "https://github.com/MystenLabs/sui",
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            BinaryName::Sui => "sui",
            BinaryName::SuiBridge => "sui-bridge",
            BinaryName::SuiFaucet => "sui-faucet",
            BinaryName::Walrus => "walrus",
            BinaryName::Mvr => "mvr",
        }
    }
}

impl std::fmt::Display for BinaryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryName::Sui => write!(f, "sui"),
            BinaryName::SuiBridge => write!(f, "sui-bridge"),
            BinaryName::SuiFaucet => write!(f, "sui-faucet"),
            BinaryName::Walrus => write!(f, "walrus"),
            BinaryName::Mvr => write!(f, "mvr"),
        }
    }
}

pub struct CommandMetadata {
    pub name: BinaryName,
    pub network: String,
    pub version: Option<String>,
}

pub fn parse_component_with_version(s: &str) -> Result<CommandMetadata, anyhow::Error> {
    let parts: Vec<&str> = s.split_whitespace().collect();

    match parts.len() {
        1 => {
            let component = BinaryName::from_str(parts[0], true)
                .map_err(|_| anyhow!("Invalid component name: {}", parts[0]))?;
            let (network, version) = parse_version_spec(None)?;
            let component_metadata = CommandMetadata {
                name: component,
                network,
                version,
            };
            Ok(component_metadata)
        }
        2 => {
            let component = BinaryName::from_str(parts[0], true)
                .map_err(|_| anyhow!("Invalid component name: {}", parts[0]))?;
            let (network, version) = parse_version_spec(Some(parts[1].to_string()))?;
            let component_metadata = CommandMetadata {
                name: component,
                network,
                version,
            };
            Ok(component_metadata)
        }
        _ => bail!("Invalid format. Use 'component' or 'component version'".to_string()),
    }
}

pub fn parse_version_spec(spec: Option<String>) -> Result<(String, Option<String>), Error> {
    match spec {
        None => Ok(("testnet".to_string(), None)),
        Some(spec) => {
            if spec.starts_with("testnet-")
                || spec.starts_with("devnet-")
                || spec.starts_with("mainnet-")
            {
                let parts: Vec<&str> = spec.splitn(2, '-').collect();
                Ok((parts[0].to_string(), Some(parts[1].to_string())))
            } else if spec == "testnet" || spec == "devnet" || spec == "mainnet" {
                Ok((spec, None))
            } else {
                // Assume it's a version for testnet
                Ok(("testnet".to_string(), Some(spec)))
            }
        }
    }
}

impl std::str::FromStr for BinaryName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sui" => Ok(BinaryName::Sui),
            "sui-bridge" => Ok(BinaryName::SuiBridge),
            "sui-faucet" => Ok(BinaryName::SuiFaucet),
            "walrus" => Ok(BinaryName::Walrus),
            "mvr" => Ok(BinaryName::Mvr),
            _ => Err(format!("Unknown component: {}", s)),
        }
    }
}
