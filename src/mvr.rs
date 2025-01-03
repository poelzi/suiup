use crate::handle_commands::{binaries_folder, download_file};
use anyhow::{anyhow, Error};
use reqwest::Client;
use serde::Deserialize;

const MVR_REPO: &str = "MystenLabs/mvr";

#[derive(Deserialize, Debug)]
pub struct MvrRelease {
    pub tag_name: String,
    pub assets: Vec<MvrAsset>,
}

#[derive(Deserialize, Debug)]
pub struct MvrAsset {
    pub name: String,
    pub browser_download_url: String,
}

pub struct MvrInstaller;

impl MvrInstaller {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_releases(&self) -> Result<Vec<MvrRelease>, Error> {
        let client = Client::new();
        let url = format!("https://api.github.com/repos/{}/releases", MVR_REPO);

        let releases: Vec<MvrRelease> = client
            .get(&url)
            .header("User-Agent", "suiup")
            .send()
            .await?
            .json()
            .await?;

        Ok(releases)
    }

    pub async fn get_latest_version(&self) -> Result<String, Error> {
        let releases = self.get_releases().await?;
        releases
            .first()
            .map(|r| r.tag_name.clone())
            .ok_or_else(|| anyhow!("No MVR releases found"))
    }

    pub fn get_binary_name() -> String {
        #[cfg(target_os = "windows")]
        {
            "mvr.exe".to_string()
        }
        #[cfg(not(target_os = "windows"))]
        {
            "mvr".to_string()
        }
    }

    pub fn get_asset_name(&self) -> String {
        let (os, arch) = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "x86_64") {
                ("macos", "x86_64")
            } else {
                ("macos", "arm64")
            }
        } else if cfg!(target_os = "windows") {
            ("windows", "x86_64")
        } else {
            // Linux/Ubuntu
            if cfg!(target_arch = "x86_64") {
                ("ubuntu", "x86_64")
            } else {
                ("ubuntu", "aarch64")
            }
        };

        let mut name = format!("mvr-{os}-{arch}");
        if cfg!(target_os = "windows") {
            name.push_str(".exe");
        }
        name
    }

    /// Download the MVR CLI binary, if it does not exist in the binary folder.
    pub async fn download_version(&self, version: Option<String>) -> Result<String, Error> {
        let version = if let Some(v) = version {
            v
        } else {
            self.get_latest_version().await?
        };

        let cache_folder = binaries_folder()?.join("standalone");
        if !cache_folder.exists() {
            std::fs::create_dir_all(&cache_folder)?;
        }
        let mvr_binary_path = cache_folder.join(format!("mvr-{}", version));
        if mvr_binary_path.exists() {
            println!("MVR v{} already installed. Use `suiup default set mvr {}` to set the default version to the desired one", version, version);
            return Ok(version);
        }

        let releases = self.get_releases().await?;
        let release = releases
            .iter()
            .find(|r| r.tag_name == version)
            .ok_or_else(|| anyhow!("Version {} not found", version))?;

        let asset_name = self.get_asset_name();
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.starts_with(&asset_name))
            .ok_or_else(|| anyhow!("No compatible binary found for your system"))?;

        download_file(
            &asset.browser_download_url,
            &mvr_binary_path,
            format!("mvr-{}", version).as_str(),
        )
        .await?;

        Ok(version)
    }
}
