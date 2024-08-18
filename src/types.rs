use anyhow::{anyhow, Error};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::handle_commands::extract_version_from_release;

pub type Version = String;

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Release {
    pub assets: Vec<Asset>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Asset {
    pub browser_download_url: String,
    pub name: String,
    size: u64,
}

pub(crate) struct Binaries {
    pub binaries: Vec<BinaryVersion>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct BinaryVersion {
    pub binary_name: String,
    pub network: Network,
    pub version: String,
}

impl Display for Binaries {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut s: HashMap<Network, Vec<(String, String)>> = HashMap::new();

        for b in self.binaries.clone() {
            if let Some(binaries) = s.get_mut(&b.network) {
                binaries.push((b.binary_name, b.version));
            } else {
                s.insert(b.network, vec![(b.binary_name, b.version)]);
            }
        }

        for (network, binaries) in s {
            writeln!(f, "[{network} release]")?;
            for (binary, version) in binaries {
                writeln!(f, "    {binary}-{version}")?;
            }
        }
        Ok(())
    }
}

impl Display for BinaryVersion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.binary_name, self.version)
    }
}

#[derive(
    Copy, Deserialize, Serialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Network {
    Testnet,
    Devnet,
    Mainnet,
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Network::Testnet => write!(f, "testnet"),
            Network::Devnet => write!(f, "devnet"),
            Network::Mainnet => write!(f, "mainnet"),
        }
    }
}

impl BinaryVersion {
    pub fn from_filename_network(filename: &str, network: &str) -> Result<Self, Error> {
        let version = extract_version_from_release(filename)?;
        let binary_name = filename.replace(&format!("-{}", version), "");
        Ok(BinaryVersion {
            binary_name,
            network: Network::from_str(network)?,
            version,
        })
    }
}

impl Network {
    fn from_str(input: &str) -> Result<Network, Error> {
        match input.to_lowercase().as_str() {
            "devnet" => Ok(Network::Devnet),
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            _ => Err(anyhow!("Invalid network")),
        }
    }
}

impl From<HashMap<String, (Network, Version)>> for Binaries {
    fn from(map: HashMap<String, (Network, Version)>) -> Self {
        let binaries = map
            .iter()
            .map(|(k, v)| BinaryVersion {
                binary_name: k.to_string(),
                network: v.0,
                version: v.1.to_string(),
            })
            .collect();
        Binaries { binaries }
    }
}
