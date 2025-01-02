use anyhow::anyhow;
use anyhow::Error;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::io::Write;
use std::path::PathBuf;
const WALRUS_BASE_URL: &str = "https://storage.googleapis.com/mysten-walrus-binaries";

pub enum WalrusArch {
    UbuntuX86_64,
    UbuntuX86_64Generic,
    MacosArm64,
    MacosX86_64,
    WindowsX86_64,
}

impl WalrusArch {
    fn to_filename(&self) -> String {
        match self {
            WalrusArch::UbuntuX86_64 => "ubuntu-x86_64",
            WalrusArch::UbuntuX86_64Generic => "ubuntu-x86_64-generic",
            WalrusArch::MacosArm64 => "macos-arm64",
            WalrusArch::MacosX86_64 => "macos-x86_64",
            WalrusArch::WindowsX86_64 => "windows-x86_64.exe",
        }
        .to_string()
    }
}

pub struct WalrusInstaller {
    arch: WalrusArch,
    install_dir: PathBuf,
}

impl WalrusInstaller {
    pub fn new(arch: WalrusArch, install_dir: &PathBuf) -> Self {
        Self {
            arch,
            install_dir: install_dir.to_path_buf(),
        }
    }

    pub fn get_download_url(&self) -> String {
        format!(
            "{}/walrus-testnet-latest-{}",
            WALRUS_BASE_URL,
            self.arch.to_filename()
        )
    }

    pub async fn download(&self) -> Result<(), Error> {
        let client = Client::new();
        let response = client
            .get(&self.get_download_url())
            .header("User-Agent", "suiup")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download Walrus binary: HTTP {}",
                response.status()
            ));
        }

        let total_size = response
            .headers()
            .get("x-goog-stored-content-length")
            .and_then(|c| c.to_str().ok())
            .and_then(|c| c.parse::<u64>().ok())
            .unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));

        println!("{}", self.install_dir.display());

        let binary_path = self.install_dir.join("walrus-latest");
        println!("Downloading Walrus binary to {:?}", binary_path);

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

pub fn detect_arch() -> Option<WalrusArch> {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Some(WalrusArch::UbuntuX86_64)
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            Some(WalrusArch::MacosArm64)
        } else if cfg!(target_arch = "x86_64") {
            Some(WalrusArch::MacosX86_64)
        } else {
            None
        }
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Some(WalrusArch::WindowsX86_64)
    } else {
        None
    }
}
