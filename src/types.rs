use anyhow::{anyhow, Error};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::handle_commands::{extract_version_from_release, installed_binaries_file};

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
    /// The name of the Sui tool binary
    pub binary_name: String,
    /// The network release of the binary
    pub network_release: Network,
    /// The version of the binary in the corresponding release
    pub version: String,
    /// Path to the binary
    pub path: Option<String>,
}

impl Display for Binaries {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut s: HashMap<Network, Vec<(String, String)>> = HashMap::new();

        for b in self.binaries.clone() {
            if let Some(binaries) = s.get_mut(&b.network_release) {
                binaries.push((b.binary_name, b.version));
            } else {
                s.insert(b.network_release, vec![(b.binary_name, b.version)]);
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
            network_release: Network::from_str(network)?,
            version,
            path: None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// Struct to store the installed binaries
pub struct InstalledBinaries {
    binaries: Vec<BinaryVersion>,
}

impl InstalledBinaries {
    pub(crate) fn create_file(path: &PathBuf) -> Result<(), Error> {
        let binaries = InstalledBinaries { binaries: vec![] };
        let s = serde_json::to_string(&binaries)
            .map_err(|_| anyhow!("Cannot serialize the installed binaries to file"))?;
        std::fs::write(path, s).map_err(|_| anyhow!("Cannot write the installed binaries file"))?;
        Ok(())
    }

    pub(crate) fn new() -> Result<Self, Error> {
        Self::read_from_file()
    }
    pub(crate) fn save_to_file(&self) -> Result<(), Error> {
        let s = serde_json::to_string(self)
            .map_err(|_| anyhow!("Cannot read the installed binaries file"))?;
        std::fs::write(installed_binaries_file()?, s)
            .map_err(|_| anyhow!("Cannot serialize the installed binaries to file"))?;
        Ok(())
    }
    pub(crate) fn read_from_file() -> Result<Self, Error> {
        let s = std::fs::read_to_string(installed_binaries_file()?)
            .map_err(|_| anyhow!("Cannot read from the installed binaries file"))?;
        let binaries: InstalledBinaries = serde_json::from_str(&s)
            .map_err(|_| anyhow!("Cannot deserialize from installed binaries file"))?;
        Ok(binaries)
    }

    pub(crate) fn add_binary(&mut self, binary: BinaryVersion) {
        self.binaries.push(binary);
    }

    pub(crate) fn remove_binary(&mut self, binary: &str) {
        self.binaries.retain(|b| b.binary_name != binary);
    }

    pub(crate) fn binaries(&self) -> &[BinaryVersion] {
        &self.binaries
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
                network_release: v.0,
                version: v.1.to_string(),
                path: None,
            })
            .collect();
        Binaries { binaries }
    }
}
