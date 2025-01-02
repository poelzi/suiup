use anyhow::{anyhow, Error};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;

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

pub struct MvrInstaller {
    install_dir: PathBuf,
}

impl MvrInstaller {
    pub fn new(install_dir: PathBuf) -> Self {
        Self { install_dir }
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
            .map(|r| r.tag_name.trim_start_matches('v').to_string())
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

    pub fn get_asset_name(&self, version: &str) -> String {
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

    pub async fn download_version(&self, version: Option<String>) -> Result<(), Error> {
        let version = if let Some(v) = version {
            v
        } else {
            self.get_latest_version().await?
        };

        let releases = self.get_releases().await?;
        let release = releases
            .iter()
            .find(|r| r.tag_name == format!("v{}", version))
            .ok_or_else(|| anyhow!("Version {} not found", version))?;

        let asset_name = self.get_asset_name(&version);
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.starts_with(&asset_name))
            .ok_or_else(|| anyhow!("No compatible binary found for your system"))?;

        let client = Client::new();
        let response = client
            .get(&asset.browser_download_url)
            .header("User-Agent", "suiup")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download MVR binary: HTTP {}",
                response.status()
            ));
        }

        let total_size = response
            .content_length()
            .ok_or_else(|| anyhow!("Failed to get content length"))?;

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));

        let binary_path = self.install_dir.join(Self::get_binary_name());
        println!("Installing MVR binary to {:?}", binary_path);

        let mut file = std::fs::File::create(&binary_path)?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
            pb.set_position(downloaded);
            file.write_all(&chunk)?;
        }

        pb.finish_with_message("Download complete");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        Ok(())
    }
}

