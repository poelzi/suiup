use anyhow::anyhow;
use anyhow::bail;
use anyhow::Error;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::HumanBytes;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::header::ETAG;
use reqwest::header::IF_NONE_MATCH;
use reqwest::header::USER_AGENT;
use reqwest::Client;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::set_permissions;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;
use tar::Archive;

use crate::commands::parse_component_with_version;
use crate::commands::ComponentCommands;
use crate::commands::DefaultCommands;
use crate::mvr;
use crate::types::Binaries;
use crate::types::BinaryVersion;
use crate::types::InstalledBinaries;
use crate::types::Network;
use crate::types::Release;
use crate::types::Version;
use crate::{
    get_config_file, get_default_bin_dir, get_suiup_cache_dir, get_suiup_config_dir,
    get_suiup_data_dir, GITHUB_REPO, RELEASES_ARCHIVES_FOLDER,
};
use clap_complete::Shell;
use std::cmp::min;
use std::env;

pub const WALRUS_BASE_URL: &str = "https://storage.googleapis.com/mysten-walrus-binaries";
fn available_components() -> &'static [&'static str] {
    &["sui", "sui-bridge", "sui-faucet", "walrus", "mvr"]
}

fn install_binary(
    name: &str,
    network: String,
    version: &str,
    debug: bool,
    binary_path: PathBuf,
    yes: bool,
) -> Result<(), Error> {
    let mut installed_binaries = InstalledBinaries::new()?;
    installed_binaries.add_binary(BinaryVersion {
        binary_name: name.to_string(),
        network_release: network.clone(),
        version: version.to_string(),
        debug,
        path: Some(binary_path.to_string_lossy().to_string()),
    });
    installed_binaries.save_to_file()?;
    update_after_install(&vec![name.to_string()], network, version, debug, yes)?;
    Ok(())
}

// this is used for sui mostly
async fn install_from_release(
    name: &str,
    network: &str,
    version_spec: Option<String>,
    debug: bool,
    yes: bool,
) -> Result<(), Error> {
    let filename = match version_spec {
        Some(version) => download_release_at_version(network, &version).await?,
        None => download_latest_release(network).await?,
    };

    let version = extract_version_from_release(&filename)?;
    let binary_name = if debug && name == "sui" {
        format!("{}-debug", name)
    } else {
        name.to_string()
    };

    if !check_if_binaries_exist(&binary_name, network.to_string(), &version)? {
        println!("Adding component: {name}-{version}");
        extract_component(&binary_name, network.to_string(), &filename)?;

        let binary_filename = format!("{}-{}", name, version);
        #[cfg(target_os = "windows")]
        let binary_filename = format!("{}.exe", binary_filename);

        let binary_path = binaries_folder()?.join(network).join(binary_filename);
        install_binary(name, network.to_string(), &version, debug, binary_path, yes)?;
    } else {
        println!("Component {name}-{version} already installed");
    }
    Ok(())
}

async fn install_from_nightly(
    name: &str,
    branch: &str,
    debug: bool,
    yes: bool,
) -> Result<(), Error> {
    println!("Installing {name} from {branch} branch");
    check_cargo_rust_installed()?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["-", "\\", "|", "/"]),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("Compiling...please wait");

    let cmd = Command::new("cargo")
        .args(&[
            "install",
            "--locked",
            "--git",
            "https://github.com/MystenLabs/sui",
            "--branch",
            branch,
            name,
            "--path",
            binaries_folder()?.join(branch).to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = cmd.wait_with_output()?;
    pb.finish_with_message("Done!");

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Error during installation:\n{}", error_message));
    }

    println!("Installation completed successfully!");
    let binary_path = binaries_folder()?.join(branch).join(name);
    install_binary(name, branch.to_string(), "nightly", debug, binary_path, yes)?;

    Ok(())
}

async fn install_walrus(network: String, yes: bool) -> Result<(), Error> {
    if !check_if_binaries_exist("walrus", network.clone(), "latest")? {
        println!("Adding component: walrus-latest");
        let (os, arch) = detect_os_arch()?;
        let download_dir = binaries_folder()?.join(network.clone());
        let download_to = download_dir.join("walrus-latest");
        download_file(
            &format!(
                "{}/walrus-{network}-latest-{os}-{arch}",
                WALRUS_BASE_URL,
                network = network
            ),
            &download_to,
            "walrus-latest",
        )
        .await?;

        let filename = "walrus";

        #[cfg(target_os = "windows")]
        let filename = format!("walrus.exe");

        install_binary(filename, network, "latest", false, binaries_folder()?, yes)?;
    } else {
        println!("Component walrus-latest already installed");
    }
    Ok(())
}

async fn install_mvr(version_spec: Option<String>, yes: bool) -> Result<(), Error> {
    let version = version_spec.clone().unwrap_or_else(|| "".to_string());
    if !check_if_binaries_exist("mvr", "standalone".to_string(), &version)? {
        let mut installer = mvr::MvrInstaller::new();
        let version = version_spec.clone();
        let installed_version = installer.download_version(version.clone()).await?;

        println!("Adding component: mvr-{installed_version}");

        let binary_path = default_bin_folder()?;
        install_binary(
            "mvr",
            "standalone".to_string(),
            &installed_version,
            false,
            binary_path,
            yes,
        )?;
    } else {
        println!("Component mvr-{version} already installed. Use `suiup default set mvr {version}` to set the default version to the specified one.");
    }

    Ok(())
}

// Main component handling function
pub(crate) async fn handle_component(cmd: ComponentCommands) -> Result<(), Error> {
    match cmd {
        ComponentCommands::List => {
            let components = available_components();
            println!("Available components:");
            for component in components {
                println!(" - {}", component);
            }
        }
        ComponentCommands::Add {
            components,
            debug,
            nightly,
            yes,
        } => {
            if components.is_empty() {
                print!("No components provided. Use `suiup component list` to see available components.");
                return Ok(());
            }

            // Ensure installation directories exist
            let default_bin_dir = get_default_bin_dir();
            std::fs::create_dir_all(&default_bin_dir)?;

            let installed_bins_dir = binaries_folder()?;
            std::fs::create_dir_all(&installed_bins_dir)?;

            let components = components.join(" ");
            let (component, version_spec) =
                parse_component_with_version(&components).map_err(|e| anyhow!("{e}"))?;

            let name = component.to_string();
            let available_components = available_components();
            if !available_components.contains(&name.as_str()) {
                println!("Component {} does not exist", name);
                return Ok(());
            }

            match (name.as_str(), &nightly) {
                ("walrus", _) => {
                    let (network, _) = parse_version_spec(version_spec)?;
                    std::fs::create_dir_all(&installed_bins_dir.join(network.clone()))?;
                    install_walrus(network, yes).await?;
                }
                ("mvr", _) => {
                    std::fs::create_dir_all(&installed_bins_dir.join("standalone"))?;
                    install_mvr(version_spec, yes).await?;
                }
                (_, Some(branch)) => {
                    install_from_nightly(&name, branch, debug, yes).await?;
                }
                _ => {
                    let (network, version) = parse_version_spec(version_spec)?;
                    install_from_release(&name, &network, version, debug, yes).await?;
                }
            }
        }
        ComponentCommands::Remove { binaries } => {
            let binaries: Vec<String> = binaries.into_iter().map(|c| c.to_string()).collect();
            // remove from default file
            // remove from default_bin
            // remove from binaries
            // REFACTOR THIS SHITTY CODE HAHAH!

            let mut installed_binaries = InstalledBinaries::new()?;

            let binaries_to_remove = installed_binaries
                .binaries()
                .iter()
                .filter(|b| binaries.contains(&b.binary_name))
                .collect::<Vec<_>>();

            for p in &binaries_to_remove {
                if let Some(p) = p.path.as_ref() {
                    if !PathBuf::from(p).exists() {
                        bail!("Binary {p} does not exist. Aborting the command.");
                    }
                }
            }
            println!("Removing binaries...");

            let default_file =
                default_file_path().map_err(|e| anyhow!("Cannot find default file: {e}"))?;
            let default_binaries = std::fs::read_to_string(&default_file)
                .map_err(|_| anyhow!("Cannot read file {}", default_file.display()))?;
            let mut default_binaries: HashMap<String, (Network, Version, bool)> =
                serde_json::from_str(&default_binaries)
                    .map_err(|_| anyhow!("Cannot decode default binary file to JSON"))?;

            // Remove the installed binaries folder
            for binary in &binaries_to_remove {
                if let Some(p) = binary.path.as_ref() {
                    std::fs::remove_file(p).map_err(|e| anyhow!("Cannot remove file: {e}"))?;
                }
            }

            // Remove the binaries from the default-bin folder
            let default_binaries_to_remove = binaries_to_remove
                .iter()
                .map(|x| &x.binary_name)
                .collect::<HashSet<_>>();
            for binary in default_binaries_to_remove {
                let default_bin_path = default_bin_folder()
                    .map_err(|e| anyhow!("Cannot find the default-bin folder: {e}"))?
                    .join(&binary);
                if default_bin_path.exists() {
                    std::fs::remove_file(default_bin_path)
                        .map_err(|e| anyhow!("Cannot remove file: {e}"))?;
                }

                default_binaries.remove(binary);
            }

            // Remove from default binaries metadata file
            File::create(&default_file)
                .map_err(|_| anyhow!("Cannot create file: {}", default_file.display()))?
                .write_all(serde_json::to_string_pretty(&default_binaries)?.as_bytes())?;

            // Remove from installed_binaries metadata file
            for binary in &binaries {
                installed_binaries.remove_binary(binary)
            }
            installed_binaries.save_to_file()?;
        }
    }
    Ok(())
}

/// Handles the default commands
pub(crate) fn handle_default(cmd: DefaultCommands) -> Result<(), Error> {
    match cmd {
        DefaultCommands::Get => {
            let default = std::fs::read_to_string(default_file_path()?)?;
            let default: HashMap<String, (String, Version, bool)> = serde_json::from_str(&default)?;
            let default_binaries = Binaries::from(default);
            println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");
        }

        DefaultCommands::Set {
            name,
            network_release,
            version,
            debug,
        } => {
            let network = network_release.unwrap_or_else(|| "testnet".to_string());

            // a map of network --> to BinaryVersion
            let installed_binaries = installed_binaries_grouped_by_network(None)?;
            let binaries = installed_binaries
                .get(&network)
                .ok_or_else(|| anyhow!("No binaries installed for {network}"))?;

            // Check if the binary exists in any network
            let binary_exists = installed_binaries
                .values()
                .any(|bins| bins.iter().any(|x| x.binary_name == name));
            if !binary_exists {
                bail!("Binary {name} not found in installed binaries. Use `suiup show` to see installed binaries.");
            }

            let version = if let Some(version) = version {
                version
            } else {
                binaries
                    .iter()
                    .filter(|b| b.binary_name == name)
                    .max_by(|a, b| a.version.cmp(&b.version))
                    .map(|b| b.version.clone())
                    .ok_or_else(|| anyhow!("No version found for {name} in {network}"))?
            };

            // check if the binary for this network and version exists
            let binary_version = format!("{}-{}", name, version);
            binaries
                .iter()
                .find(|b| {
                    b.binary_name == name && b.version == version && b.network_release == network
                })
                .ok_or_else(|| {
                    anyhow!("Binary {binary_version} from {network} release not found. Use `suiup show` to see installed binaries.")
                })?;

            // copy files to default-bin
            let mut dst = default_bin_folder()?;
            #[cfg(target_os = "windows")]
            {
                let name = if debug {
                    format!("{}-debug.exe", name)
                } else {
                    format!("{}.exe", name)
                };
            }
            dst.push(&name);

            let mut src = binaries_folder()?;
            src.push(network.to_string());
            #[cfg(target_os = "windows")]
            {
                let binary_version = format!("{}.exe", binary_version);
            }
            if debug {
                src.push(format!("{}-debug-{}", name, version));
            } else {
                src.push(binary_version);
            }

            #[cfg(not(target_os = "windows"))]
            {
                if dst.exists() {
                    std::fs::remove_file(&dst)?;
                }
                std::os::unix::fs::symlink(&src, &dst)?;
            }

            #[cfg(target_os = "windows")]
            {
                std::fs::copy(&src, &dst)?;
            }

            update_default_version_file(&vec![name], network, &version, debug)?;
            println!("Default binary updated successfully");
        }
    }

    Ok(())
}

/// Handles the `show` command
pub fn handle_show() -> Result<(), Error> {
    let default = std::fs::read_to_string(default_file_path()?)?;
    let default: HashMap<String, (String, Version, bool)> = serde_json::from_str(&default)?;
    let default_binaries = Binaries::from(default);

    println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");

    let installed_binaries = installed_binaries_grouped_by_network(None)?;

    println!("\x1b[1mInstalled binaries:\x1b[0m");

    for (network, binaries) in installed_binaries {
        println!("[{network} release]");
        for binary in binaries {
            println!("    {binary}");
        }
    }

    Ok(())
}

/// Handles the `update` command
pub async fn handle_update(binary_name: String) -> Result<(), Error> {
    if !available_components().contains(&binary_name.as_str()) {
        bail!("Invalid component name: {}", binary_name);
    }

    let installed_binaries = InstalledBinaries::new()?;
    let binaries = installed_binaries.binaries();
    if !binaries.iter().any(|x| x.binary_name == binary_name) {
        bail!("Binary {binary_name} not found in installed binaries. Use `suiup show` to see installed binaries and `suiup install` to install the binary.")
    }
    let binaries_by_network = installed_binaries_grouped_by_network(Some(installed_binaries))?;

    let mut network_local_last_version: Vec<(String, String)> = vec![];

    for (network, binaries) in &binaries_by_network {
        let last_version = binaries
            .iter()
            .filter(|x| x.binary_name == binary_name)
            .collect::<Vec<_>>();
        if last_version.is_empty() {
            continue;
        }
        let last_version = if last_version.len() > 1 {
            last_version
                .iter()
                .max_by(|a, b| a.version.cmp(&b.version))
                .unwrap()
        } else {
            last_version.first().unwrap()
        };
        network_local_last_version.push((network.clone(), last_version.version.clone()));
    }
    // map of network and last version known locally

    // find the last local version of the name binary, for each network
    // then find the last release for each network and compare the versions
    println!("{:?}", network_local_last_version);

    if binary_name == "mvr" {
        handle_component(ComponentCommands::Add {
            components: vec![binary_name.clone()],
            debug: false,
            nightly: None,
            yes: true,
        })
        .await?;
        return Ok(());
    }

    if binary_name == "walrus" {
        handle_component(ComponentCommands::Add {
            components: vec![binary_name.clone()],
            debug: false,
            nightly: None,
            yes: true,
        })
        .await?;
        return Ok(());
    }

    let releases = release_list().await?.0;
    let mut to_update = vec![];
    for (n, v) in &network_local_last_version {
        let last_release = last_release_for_network(&releases, &n).await?;
        let last_version = last_release.1;
        if v == &last_version {
            println!("[{n} release] {binary_name} is up to date");
        } else {
            println!("[{n} release] {binary_name} is outdated. Local: {v}, Latest: {last_version}");
            to_update.push((n, last_version));
        }
    }

    for (n, v) in to_update.iter() {
        println!("Updating {binary_name} to {v} from {n} release");
        handle_component(ComponentCommands::Add {
            components: vec![binary_name.clone()],
            debug: false,
            nightly: None,
            yes: true,
        })
        .await?;
    }

    Ok(())
}

/// Handles the `which` command
pub fn handle_which() -> Result<(), Error> {
    let default_bin = default_bin_folder()?;
    println!("Default binaries path: {}", default_bin.display());
    // Implement displaying the path to the active CLI binary here
    Ok(())
}

/// Fetches the list of releases from the GitHub repository
pub(crate) async fn release_list() -> Result<(Vec<Release>, Option<String>), anyhow::Error> {
    let client = Client::new();
    let release_url = format!("https://api.github.com/repos/{}/releases", GITHUB_REPO);
    let mut request = client.get(&release_url).header("User-Agent", "suiup");
    // Add ETag for caching
    if let Ok(etag) = read_etag_file() {
        request = request.header(IF_NONE_MATCH, etag);
    }

    let response = request.send().await?;

    // note this only works with authenticated requests. Should add support for that later.
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        // If nothing has changed, return an empty list and the existing ETag
        if let Some((releases, etag)) = load_cached_release_list()
            .map_err(|e| anyhow!("Cannot load release list from cache: {e}"))?
        {
            return Ok((releases, Some(etag)));
        }
    }

    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let releases: Vec<Release> = response.json().await?;
    save_release_list(&releases, etag.clone())?;

    Ok((releases, etag))
}

fn cache_path() -> PathBuf {
    let cache_dir = dirs::cache_dir().expect("Could not find cache directory");
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir.join("suiup"))
            .expect("Could not create cache directory");
    }
    cache_dir.join("suiup")
}

fn load_cached_release_list() -> Result<Option<(Vec<Release>, String)>, anyhow::Error> {
    let cache_file = cache_path().join("releases.json");
    let etag_file = cache_path().join("etag.txt");

    if cache_file.exists() && etag_file.exists() {
        println!("Loading releases list from cache");
        let cache_content: Vec<Release> = serde_json::from_str(
            &std::fs::read_to_string(&cache_file)
                .map_err(|_| anyhow!("Cannot read from file {}", cache_file.display()))?,
        )
        .map_err(|_| {
            anyhow!(
                "Cannot deserialize the releases cached file {}",
                cache_file.display()
            )
        })?;
        let etag_content = std::fs::read_to_string(&etag_file)
            .map_err(|_| anyhow!("Cannot read from file {}", etag_file.display()))?;

        Ok(Some((cache_content, etag_content)))
    } else {
        Ok(None)
    }
}

fn save_release_list(releases: &[Release], etag: Option<String>) -> Result<(), anyhow::Error> {
    println!("Saving releases list to cache");
    let cache_dir = cache_path();
    std::fs::create_dir_all(&cache_dir).expect("Could not create cache directory");

    let cache_file = cache_dir.join("releases.json");
    let etag_file = cache_dir.join("etag.txt");

    let cache_content =
        serde_json::to_string_pretty(releases).expect("Could not serialize releases file: {}");

    std::fs::write(&cache_file, cache_content).map_err(|_| {
        anyhow!(
            "Could not write cache releases file: {}",
            cache_file.display(),
        )
    })?;
    if let Some(etag) = etag {
        std::fs::write(&etag_file, etag)
            .map_err(|_| anyhow!("Could not write ETag file: {}", etag_file.display()))?;
    }
    Ok(())
}

fn read_etag_file() -> Result<String, anyhow::Error> {
    let etag_file = cache_path().join("etag.txt");
    if etag_file.exists() {
        let etag = std::fs::read_to_string(&etag_file)
            .map_err(|_| anyhow!("Cannot read from file {}", etag_file.display()));
        etag
    } else {
        Ok("".to_string())
    }
}

/// Finds the last release for a given network
async fn find_last_release_by_network(releases: Vec<Release>, network: &str) -> Option<Release> {
    releases
        .into_iter()
        .find(|r| r.assets.iter().any(|a| a.name.contains(network)))
}

/// Detects the current OS and architecture
pub fn detect_os_arch() -> Result<(String, String), Error> {
    let os = match whoami::platform() {
        whoami::Platform::Linux => "linux",
        whoami::Platform::Windows => "windows",
        whoami::Platform::MacOS => "macos",
        _ => anyhow::bail!("Unsupported OS. Supported only: Linux, Windows, MacOS"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        _ => anyhow::bail!("Unsupported architecture. Supported only: x86_64, aarch64"),
    };

    println!("Detected: {os}-{arch}...");
    Ok((os.to_string(), arch.to_string()))
}

/// Downloads a release with a specific version
/// The network is used to filter the release
async fn download_release_at_version(
    network: &str,
    version: &str,
) -> Result<String, anyhow::Error> {
    let (os, arch) = detect_os_arch()?;

    let tag = format!("{}-{}", network, version);

    println!("Searching for release with tag: {}...", tag);
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();

    let releases = release_list().await?.0;

    if let Some(release) = releases
        .iter()
        .find(|r| r.assets.iter().any(|a| a.name.contains(&tag)))
    {
        download_asset_from_github(release, &os, &arch).await
    } else {
        headers.insert(USER_AGENT, HeaderValue::from_static("suiup"));

        let url = format!(
            "https://api.github.com/repos/{GITHUB_REPO}/releases/tags/{}",
            tag
        );
        let response = client.get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("release {tag} not found");
        }

        let release: Release = response.json().await?;
        download_asset_from_github(&release, &os, &arch).await
    }
}

/// Downloads the latest release for a given network
async fn download_latest_release(network: &str) -> Result<String, anyhow::Error> {
    println!("Downloading release list");
    let releases = release_list().await?;

    let (os, arch) = detect_os_arch()?;

    let last_release = find_last_release_by_network(releases.0, network)
        .await
        .ok_or_else(|| anyhow!("Could not find last release"))?;

    println!(
        "Last {network} release: {}",
        extract_version_from_release(&last_release.assets[0].name)?
    );

    download_asset_from_github(&last_release, &os, &arch).await
}

pub async fn download_file(url: &str, download_to: &PathBuf, name: &str) -> Result<String, Error> {
    let client = Client::new();
    let response = client.get(url).header("User-Agent", "suiup").send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to download: {}", response.status()));
    }

    let mut total_size = response.content_length().unwrap_or_else(|| 0);
    //walrus is on google storage, so different content length header
    if total_size == 0 {
        total_size = response
            .headers()
            .get("x-goog-stored-content-length")
            .and_then(|c| c.to_str().ok())
            .and_then(|c| c.parse::<u64>().ok())
            .unwrap_or(0);
    }

    if download_to.exists() {
        if download_to.metadata()?.len() == total_size {
            println!("Found {name} in cache");
            return Ok(name.to_string());
        } else {
            std::fs::remove_file(&download_to)?;
        }
    }

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
            .template("Downloading release: {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("=>-"));

    let mut file = std::fs::File::create(&download_to)?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let start = Instant::now();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk)?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);

        let elapsed = start.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            let speed = downloaded as f64 / elapsed;
            pb.set_message(format!("Speed: {}/s", HumanBytes(speed as u64)));
        }
    }

    pb.finish_with_message("Download complete");

    Ok(name.to_string())
}

/// Downloads the archived release from GitHub and returns the file name
/// The `network, os, and arch` parameters are used to retrieve the correct release for the target
/// architecture and OS
async fn download_asset_from_github(
    release: &Release,
    os: &str,
    arch: &str,
) -> Result<String, anyhow::Error> {
    let asset = release
        .assets
        .iter()
        .find(|&a| a.name.contains(arch) && a.name.contains(os.to_string().to_lowercase().as_str()))
        .ok_or_else(|| anyhow!("Asset not found for {os}-{arch}"))?;

    let url = asset.clone().browser_download_url;
    let name = asset.clone().name;
    let path = release_archive_folder()?;
    let mut file_path = path.clone();
    file_path.push(&asset.name);

    download_file(&url, &file_path, &name).await
}

/// Extracts a component from the release archive. The component's name is identified by the
/// `binary` parameter.
///
/// This extracts the component to the binaries folder under the network from which release comes
/// from, and sets the correct permissions for Unix based systems.
fn extract_component(binary: &str, network: String, filename: &str) -> Result<(), Error> {
    let mut archive_path = release_archive_folder()?;
    archive_path.push(filename);

    let file = File::open(archive_path.as_path())
        .map_err(|_| anyhow!("Cannot open archive file: {}", archive_path.display()))?;
    let tar = GzDecoder::new(file);
    let mut archive = Archive::new(tar);

    #[cfg(target_os = "windows")]
    let binary = format!("{}.exe", binary);

    // Check if the current entry matches the file name
    for file in archive.entries()? {
        let mut f = file.unwrap();
        if f.path()?.file_name() == Some(std::ffi::OsStr::new(&binary)) {
            println!("Extracting file: {}", &binary);

            let mut output_path = binaries_folder()?;
            output_path.push(network.to_string());
            if !output_path.is_dir() {
                std::fs::create_dir_all(output_path.as_path())?;
            }
            let binary_version = format!("{}-{}", binary, extract_version_from_release(filename)?);
            output_path.push(&binary_version);
            let mut output_file = File::create(&output_path)?;

            std::io::copy(&mut f, &mut output_file)?;
            println!(" '{}' extracted successfully!", &binary);
            #[cfg(not(target_os = "windows"))]
            {
                // Retrieve and apply the original file permissions on Unix-like systems
                if let Ok(permissions) = f.header().mode() {
                    set_permissions(output_path, PermissionsExt::from_mode(permissions))?;
                }
            }
            break;
        }
    }

    Ok(())
}

/// Extracts the version from a release filename
pub(crate) fn extract_version_from_release(release: &str) -> Result<String, Error> {
    let re = regex::Regex::new(r"v\d+\.\d+\.\d+").unwrap();
    let captures = re
        .captures(release)
        .ok_or_else(|| anyhow!("Could not extract version from release"))?;

    Ok(captures.get(0).unwrap().as_str().to_string())
}

/// Returns the path to the Suiup configuration folder. The folder is created if it does not exist.
fn config_folder_or_create() -> Result<PathBuf, anyhow::Error> {
    let path = get_suiup_config_dir();
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Returns the path to the default binaries folder. The folder is created if it does not exist.
fn default_bin_folder() -> Result<PathBuf, anyhow::Error> {
    let path = get_default_bin_dir();
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Returns the path to the releases archives folder. The folder is created if it does not exist.
/// This is used to cache the release archives.
pub fn release_archive_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = get_suiup_cache_dir();
    path.push(RELEASES_ARCHIVES_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Returns the path to the binaries folder. The folder is created if it does not exist.
pub fn binaries_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = get_suiup_data_dir();
    path.push("binaries");
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Prompts the user and asks if they want to update the default version with the one that was just
/// installed.
fn update_after_install(
    name: &Vec<String>,
    network: String,
    version: &str,
    debug: bool,
    yes: bool,
) -> Result<(), Error> {
    let input = if yes {
        "y".to_string()
    } else {
        let prompt = "Do you want to set this new installed version as the default one? [y/N] ";

        print!("{prompt}");
        std::io::stdout().flush().unwrap();

        // Create a mutable String to store the input
        let mut input = String::new();

        // Read the input from the console
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        // Trim the input and convert to lowercase for comparison
        input.trim().to_lowercase()
    };

    // Check the user's response
    match input.as_str() {
        "y" | "yes" => {
            for binary in name {
                let mut filename = if debug {
                    format!("{}-debug-{}", binary, version)
                } else {
                    format!("{}-{}", binary, version)
                };

                if version.is_empty() {
                    filename = filename.strip_suffix('-').unwrap_or_default().to_string();
                }

                println!("Setting {binary}-{version} as default");
                println!("{}/{}/{}", binaries_folder()?.display(), network, filename);

                #[cfg(target_os = "windows")]
                {
                    let filename = format!("{}.exe", filename);
                }
                let src = binaries_folder()?.join(network.to_string()).join(filename);
                let dst = default_bin_folder()?.join(binary);

                std::fs::copy(&src, &dst)?;

                #[cfg(unix)]
                {
                    let mut perms = std::fs::metadata(&dst)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&dst, perms)?;
                }

                println!("[{network}] {binary}-{version} set as default");
            }
            update_default_version_file(name, network, version, debug)?;
            check_path_and_warn()?;
        }

        "" | "n" | "no" => {
            println!("Keeping the current default version.");
        }
        _ => {
            println!("Invalid input. Please enter 'y' or 'n'.");
            update_after_install(name, network, version, debug, yes)?;
        }
    }
    Ok(())
}

/// Updates the default version file with the new installed version.
fn update_default_version_file(
    binaries: &Vec<String>,
    network: String,
    version: &str,
    debug: bool,
) -> Result<(), Error> {
    let path = default_file_path()?;
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    println!("Updating default version file...");
    let mut map: HashMap<String, (String, Version, bool)> = serde_json::from_reader(reader)?;

    for binary in binaries {
        let b = map.get_mut(binary);
        if let Some(b) = b {
            b.0 = network.clone();
            b.1 = version.to_string();
            b.2 = debug;
        } else {
            map.insert(
                binary.to_string(),
                (network.clone(), version.to_string(), debug),
            );
        }
    }

    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(&map)?.as_bytes())?;

    Ok(())
}

/// Checks if the binaries exist in the binaries folder
fn check_if_binaries_exist(binary: &str, network: String, version: &str) -> Result<bool, Error> {
    let mut path = binaries_folder()?;
    path.push(network.to_string());

    let binary_version = if version.is_empty() {
        format!("{}", binary)
    } else {
        format!("{}-{}", binary, version)
    };

    #[cfg(target_os = "windows")]
    {
        path.push(format!("{}.exe", binary_version));
    }
    path.push(&binary_version);
    Ok(path.exists())
}

/// Returns the path to the default version file. The file is created if it does not exist.
fn default_file_path() -> Result<PathBuf, Error> {
    let path = get_config_file("default_version.json");
    if !path.exists() {
        let mut file = File::create(&path)?;
        let default = HashMap::<String, (String, String)>::new();
        let default_str = serde_json::to_string_pretty(&default)?;
        file.write_all(default_str.as_bytes())?;
    }
    Ok(path)
}

/// Returns a map of installed binaries grouped by network releases
fn installed_binaries_grouped_by_network(
    installed_binaries: Option<InstalledBinaries>,
) -> Result<HashMap<String, Vec<BinaryVersion>>, Error> {
    let installed_binaries = if let Some(installed_binaries) = installed_binaries {
        installed_binaries
    } else {
        InstalledBinaries::new()?
    };
    let binaries = installed_binaries.binaries();
    let mut files_by_folder: HashMap<String, Vec<BinaryVersion>> = HashMap::new();

    for b in binaries {
        if let Some(f) = files_by_folder.get_mut(&b.network_release.to_string()) {
            f.push(b.clone());
        } else {
            files_by_folder.insert(b.network_release.to_string(), vec![b.clone()]);
        }
    }

    Ok(files_by_folder)
}

pub(crate) fn installed_binaries_file() -> Result<PathBuf, Error> {
    let path = get_config_file("installed_binaries.json");
    if !path.exists() {
        InstalledBinaries::create_file(&path)?;
    }
    Ok(path)
}

pub(crate) fn initialize() -> Result<(), Error> {
    std::fs::create_dir_all(get_suiup_config_dir())?;
    std::fs::create_dir_all(get_suiup_data_dir())?;
    std::fs::create_dir_all(get_suiup_cache_dir())?;
    binaries_folder()?;
    release_archive_folder()?;
    default_bin_folder()?;
    default_file_path()?;
    installed_binaries_file()?;
    Ok(())
}

pub(crate) async fn last_release_for_network<'a>(
    releases: &'a [Release],
    network: &'a str,
) -> Result<(&'a str, String), Error> {
    if let Some(release) = releases
        .iter()
        .find(|r| r.assets.iter().any(|a| a.name.contains(network)))
    {
        Ok((
            network,
            extract_version_from_release(release.assets[0].name.as_str())?,
        ))
    } else {
        bail!("No release found for {network}")
    }
}

fn check_cargo_rust_installed() -> Result<(), Error> {
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        if output.status.success() {
            print!(
                "Rust is installed: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        } else {
            bail!("Rust is not installed");
        }
    } else {
        bail!("Failed to execute rustc command");
    }

    // Check if cargo is installed
    if let Ok(output) = Command::new("cargo").arg("--version").output() {
        if output.status.success() {
            print!(
                "Cargo is installed: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        } else {
            bail!("Cargo is not installed");
        }
    } else {
        bail!("Failed to execute cargo command");
    }
    Ok(())
}

fn check_path_and_warn() -> Result<(), Error> {
    let local_bin = get_default_bin_dir();

    // Check if the bin directory exists in PATH
    if let Ok(path) = env::var("PATH") {
        #[cfg(windows)]
        let path_separator = ';';
        #[cfg(not(windows))]
        let path_separator = ':';

        if !path
            .split(path_separator)
            .any(|p| PathBuf::from(p) == local_bin)
        {
            println!("\nWARNING: {} is not in your PATH", local_bin.display());

            #[cfg(windows)]
            {
                println!("\nTo add it to your PATH:");
                println!("1. Press Win + X and select 'System'");
                println!("2. Click on 'Advanced system settings'");
                println!("3. Click on 'Environment Variables'");
                println!("4. Under 'User variables', find and select 'Path'");
                println!("5. Click 'Edit'");
                println!("6. Click 'New'");
                println!("7. Add the following path:");
                println!("    %USERPROFILE%\\.local\\bin");
                println!("8. Click 'OK' on all windows");
                println!("9. Restart your terminal\n");
            }

            #[cfg(not(windows))]
            {
                println!("Add one of the following lines depending on your shell:");
                println!("\nFor bash/zsh (~/.bashrc or ~/.zshrc):");
                println!("    export PATH=\"$HOME/.local/bin:$PATH\"");
                println!("\nFor fish (~/.config/fish/config.fish):");
                println!("    fish_add_path $HOME/.local/bin");
                println!("\nThen restart your shell or run one of:");
                println!("    source ~/.bashrc        # for bash");
                println!("    source ~/.zshrc         # for zsh");
                println!("    source ~/.config/fish/config.fish  # for fish\n");
            }
        }
    }
    Ok(())
}

pub(crate) fn print_completion_instructions(shell: &Shell) {
    match shell {
        Shell::Bash => {
            println!("\nTo install bash completions:");
            println!("1. Create completion directory if it doesn't exist:");
            println!("    mkdir -p ~/.local/share/bash-completion/completions");
            println!("2. Add completions to the directory:");
            println!(
                "    suiup completion bash > ~/.local/share/bash-completion/completions/suiup"
            );
            println!("\nMake sure you have bash-completion installed and loaded in your ~/.bashrc");
        }
        Shell::Fish => {
            println!("\nTo install fish completions:");
            println!("1. Create completion directory if it doesn't exist:");
            println!("    mkdir -p ~/.config/fish/completions");
            println!("2. Add completions to the directory:");
            println!("    suiup completion fish > ~/.config/fish/completions/suiup.fish");
        }
        Shell::Zsh => {
            println!("\nTo install zsh completions:");
            println!("1. Create completion directory if it doesn't exist:");
            println!("    mkdir -p ~/.zsh/completions");
            println!("2. Add completions to the directory:");
            println!("    suiup completion zsh > ~/.zsh/completions/_suiup");
            println!("3. Add the following to your ~/.zshrc:");
            println!("    fpath=(~/.zsh/completions $fpath)");
            println!("    autoload -U compinit; compinit");
        }
        _ => {}
    }
}

fn parse_version_spec(spec: Option<String>) -> Result<(String, Option<String>), Error> {
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
