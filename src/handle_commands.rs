use anyhow::anyhow;
use anyhow::bail;
use anyhow::Error;
use flate2::read::GzDecoder;
use reqwest::Client;
use std::collections::HashMap;
use std::fs::set_permissions;
use std::fs::{self, File};
use std::io::BufReader;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tar::Archive;
use trauma::download::Download;
use trauma::downloader::DownloaderBuilder;

use crate::commands::ComponentCommands;
use crate::commands::DefaultCommands;
use crate::types::Binaries;
use crate::types::BinaryVersion;
use crate::types::InstalledBinaries;
use crate::types::Network;
use crate::types::Release;
use crate::types::Version;
use crate::BINARIES_FOLDER;
use crate::DEFAULT_BIN_FOLDER;
use crate::DEFAULT_VERSION_FILENAME;
use crate::GITHUB_REPO;
use crate::INSTALLED_BINARIES_FILENAME;
use crate::RELEASES_ARCHIVES_FOLDER;
use crate::SUIUP_FOLDER;

fn available_components() -> &'static [&'static str] {
    &["sui", "sui-bridge", "sui-faucet", "walrus"]
}

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
            name,
            network_release: network,
            version,
        } => {
            if name.is_empty() {
                print!("No components provided. Use `suiup component list` to see available components.");
                return Ok(());
            }

            let available_components = available_components();
            for c in &name {
                if !available_components.contains(&c.as_str()) {
                    println!("Component {} does not exist", c);
                    return Ok(());
                }
            }

            let filename = if let Some(version) = version {
                download_release_at_version(&network.to_string(), &version).await?
            } else {
                download_latest_release(&network.to_string())
                    .await
                    .map_err(|_| anyhow!("Could not download latest release"))?
            };
            let version = extract_version_from_release(&filename)?;
            let mut installed_binaries = InstalledBinaries::new()?;
            for binary in &name {
                if !check_if_binaries_exist(binary, &network, &version)? {
                    println!("Adding component: {binary}-{version}");
                    extract_component(binary, network, &filename)
                        .map_err(|_| anyhow!("Could not extract component"))?;
                    let filename = format!("{}-{}", binary, version);
                    #[cfg(target_os = "windows")]
                    {
                        let filename = format!("{}.exe", filename);
                    }
                    let binary_path = binaries_folder()?.join(network.to_string()).join(filename);
                    installed_binaries.add_binary(BinaryVersion {
                        binary_name: binary.to_string(),
                        network_release: network,
                        version: version.clone(),
                        path: Some(binary_path.to_string_lossy().to_string()),
                    });
                } else {
                    println!("Component {binary}-{version} already installed");
                }
            }
            installed_binaries.save_to_file()?;
            update_after_install(&name, network, &version)?;
        }
        ComponentCommands::Remove { binaries } => {
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
                        bail!("Binary {p} does not exist");
                    }
                }
            }

            let default_file = default_file_path()?;
            let default_binaries = std::fs::read_to_string(&default_file)?;
            let mut default_binaries: HashMap<String, (Network, Version)> =
                serde_json::from_str(&default_binaries)?;

            for binary in binaries_to_remove {
                if let Some(p) = binary.path.as_ref() {
                    std::fs::remove_file(p)?;
                    std::fs::remove_file(default_bin_folder()?.join(&binary.binary_name))?;
                    default_binaries.remove(&binary.binary_name);
                }
            }
            File::create(&default_file)?
                .write_all(serde_json::to_string(&default_binaries)?.as_bytes())?;

            // remove from installed_binaries
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
            let default: HashMap<String, (Network, Version)> = serde_json::from_str(&default)?;
            let default_binaries = Binaries::from(default);
            println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");
        }

        DefaultCommands::Set {
            name,
            network_release: network,
            version,
        } => {
            // a map of network --> to BinaryVersion
            let installed_binaries = installed_binaries_grouped_by_network()?;
            let binaries = installed_binaries.get(&network.to_string());
            if let Some(binaries) = binaries {
                for binary in &name {
                    let b = BinaryVersion {
                        binary_name: binary.to_string(),
                        network_release: network,
                        version: version.to_string(),
                        path: None,
                    };
                    if !binaries.contains(&b) {
                        println!(
                            "Component {binary}-{version} from {network} release is not installed. Use suiup show to see installed components or suiup component add {binary} --network {network} --version {version} to install it."
                        );
                        return Ok(());
                    }
                }
            } else {
                println!("No components installed from {network} release");
                return Ok(());
            }
            // copy files to default-bin
            for n in &name {
                let mut dst = default_bin_folder()?;
                #[cfg(target_os = "windows")]
                {
                    let n = format!("{}.exe", n);
                }
                dst.push(n);

                let mut src = binaries_folder()?;
                src.push(network.to_string());
                let binary_version = format!("{}-{}", n, version);
                #[cfg(target_os = "windows")]
                {
                    let binary_version = format!("{}.exe", binary_version);
                }
                src.push(binary_version);

                std::fs::copy(src, dst)?;
            }
            update_default_version_file(&name, network, &version)?
        }
    }

    Ok(())
}

/// Handles the `show` command
pub fn handle_show() -> Result<(), Error> {
    let default = std::fs::read_to_string(default_file_path()?)?;
    let default: HashMap<String, (Network, Version)> = serde_json::from_str(&default)?;
    let default_binaries = Binaries::from(default);

    println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");

    let installed_binaries = installed_binaries_grouped_by_network()?;

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
pub fn handle_update() {
    println!("Updating components...");
    // Implement the update logic here
}

/// Handles the `override` command
pub fn handle_override() {
    println!("Overriding project’s CLI version...");
    // Implement the override logic here
}

/// Handles the `which` command
pub fn handle_which() -> Result<(), Error> {
    let default_bin = default_bin_folder()?;
    println!("Default binaries path: {}", default_bin.display());
    // Implement displaying the path to the active CLI binary here
    Ok(())
}

/// Fetches the list of releases from the GitHub repository
async fn release_list() -> Result<Vec<Release>, anyhow::Error> {
    let client = Client::new();
    let release_url = format!("https://api.github.com/repos/{}/releases", GITHUB_REPO);
    let releases: Vec<Release> = client
        .get(&release_url)
        .header("User-Agent", "suiup")
        .send()
        .await?
        .json()
        .await?;

    Ok(releases)
}

/// Finds the last release for a given network
async fn find_last_release_by_network(releases: Vec<Release>, network: &str) -> Option<Release> {
    releases
        .into_iter()
        .find(|r| r.assets.iter().any(|a| a.name.contains(network)))
}

/// Detects the current OS and architecture
fn detect_os_arch() -> Result<(String, String), Error> {
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
    let client = Client::new();
    let release_url = format!(
        "https://api.github.com/repos/{}/releases/tags/{tag}",
        GITHUB_REPO
    );
    let response = client
        .get(&release_url)
        .header("User-Agent", "suiup")
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!("release {tag} not found");
    }

    let release: Release = response.json().await?;
    download_asset_from_github(&release, network, &os, &arch).await
}

/// Downloads the latest release for a given network
async fn download_latest_release(network: &str) -> Result<String, anyhow::Error> {
    let releases = release_list().await?;

    let (os, arch) = detect_os_arch()?;

    let last_release = find_last_release_by_network(releases, network)
        .await
        .ok_or_else(|| anyhow!("Could not find last release"))?;

    println!(
        "Last {network} release: {}",
        extract_version_from_release(&last_release.assets[0].name)?
    );

    download_asset_from_github(&last_release, network, &os, &arch).await
}

/// Downloads the archived release from GitHub and returns the file name
/// The `network, os, and arch` parameters are used to retrieve the correct release for the target
/// architecture and OS
async fn download_asset_from_github(
    release: &Release,
    network: &str,
    os: &str,
    arch: &str,
) -> Result<String, anyhow::Error> {
    let asset = release
        .assets
        .iter()
        .find(|&a| a.name.contains(arch) && a.name.contains(os.to_string().to_lowercase().as_str()))
        .ok_or_else(|| anyhow!("Asset not found"))?;

    // Find the archive file for the current OS and architecture

    let path = release_archive_folder()?;
    let mut file_path = path.clone();
    file_path.push(&asset.name);
    if file_path.exists() {
        println!("Found release archive {} in cache", asset.name);
        return Ok(asset.name.to_string());
    }

    println!("Downloading {network} release: {}...", asset.name);
    let downloads = vec![Download::try_from(asset.browser_download_url.as_str()).unwrap()];
    let downloader = DownloaderBuilder::new().directory(path).build();
    downloader.download(&downloads).await;

    Ok(asset.name.to_string())
}

/// Extracts a component from the release archive. The component's name is identified by the
/// `binary` parameter.
///
/// This extracts the component to the binaries folder under the network from which release comes
/// from, and sets the correct permissions for Unix based systems.
fn extract_component(binary: &str, network: Network, filename: &str) -> Result<(), Error> {
    let mut archive_path = release_archive_folder()?;
    archive_path.push(filename);

    let file = File::open(archive_path.as_path())?;
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
    let mut path = std::path::PathBuf::new();

    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    path.push(home_dir);
    path.push(SUIUP_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }

    Ok(path)
}

/// Returns the path to the default binaries folder. The folder is created if it does not exist.
fn default_bin_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = config_folder_or_create()?;
    path.push(DEFAULT_BIN_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Returns the path to the releases archives folder. The folder is created if it does not exist.
/// This is used to cache the release archives.
fn release_archive_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = config_folder_or_create()?;
    path.push(RELEASES_ARCHIVES_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Returns the path to the binaries folder. The folder is created if it does not exist.
fn binaries_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = config_folder_or_create()?;
    path.push(BINARIES_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

/// Prompts the user and asks if they want to update the default version with the one that was just
/// installed.
fn update_after_install(name: &Vec<String>, network: Network, version: &str) -> Result<(), Error> {
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
    let input = input.trim().to_lowercase();

    // Check the user's response
    match input.as_str() {
        "y" | "yes" => {
            for binary in name {
                let filename = format!("{}-{}", binary, version);
                #[cfg(target_os = "windows")]
                {
                    let filename = format!("{}.exe", filename);
                }
                let mut src = binaries_folder()?;
                src.push(network.to_string());
                src.push(filename);

                let mut dst = default_bin_folder()?;
                dst.push(binary);

                std::fs::copy(src, dst)?;

                println!("[{network}] {binary}-{version} set as default");
            }
            update_default_version_file(name, network, version)?;
        }

        "" | "n" | "no" => {
            println!("Keeping the current default version.");
        }
        _ => {
            println!("Invalid input. Please enter 'y' or 'n'.");
            update_after_install(name, network, version)?;
        }
    }
    Ok(())
}

/// Updates the default version file with the new installed version.
fn update_default_version_file(
    binaries: &Vec<String>,
    network: Network,
    version: &str,
) -> Result<(), Error> {
    let path = default_file_path()?;
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    println!("Updating default version file...");
    let mut map: HashMap<String, (Network, Version)> = serde_json::from_reader(reader)?;

    for binary in binaries {
        let b = map.get_mut(binary);
        if let Some(b) = b {
            b.0 = network;
            b.1 = version.to_string();
        } else {
            map.insert(binary.to_string(), (network, version.to_string()));
        }
    }

    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string(&map)?.as_bytes())?;

    Ok(())
}

/// Checks if the binaries exist in the binaries folder
fn check_if_binaries_exist(binary: &str, network: &Network, version: &str) -> Result<bool, Error> {
    let mut path = binaries_folder()?;
    path.push(network.to_string());
    let binary_version = format!("{}-{}", binary, version);
    #[cfg(target_os = "windows")]
    {
        path.push(format!("{}.exe", binary_version));
    }
    path.push(&binary_version);
    Ok(path.exists())
}

/// Returns the path to the default version file. The file is created if it does not exist.
fn default_file_path() -> Result<PathBuf, Error> {
    let mut path = config_folder_or_create()?;
    path.push(DEFAULT_VERSION_FILENAME);
    if !path.exists() {
        let mut file = File::create(&path)?;
        let default = HashMap::<String, (String, String)>::new();
        let default_str = serde_json::to_string(&default)?;
        file.write_all(serde_json::to_string(&default_str)?.as_bytes())?;
    }
    Ok(path)
}

/// Returns a map of installed binaries grouped by network releases
fn installed_binaries_grouped_by_network() -> Result<HashMap<String, Vec<BinaryVersion>>, Error> {
    let mut files_by_folder: HashMap<String, Vec<BinaryVersion>> = HashMap::new();
    let mut folders = Vec::new();

    // Read the directory contents
    for entry in fs::read_dir(binaries_folder()?)? {
        let entry = entry?;
        let path = entry.path();

        // Check if the path is a directory
        if path.is_dir() {
            folders.push(path);
        }
    }

    for network_folder in &folders {
        let paths = fs::read_dir(network_folder)?
            .filter_map(Result::ok) // Filter out errors, if any
            .filter(|entry| entry.path().is_file())
            .collect::<Vec<_>>();

        for p in paths {
            if let Some(filename) = p.path().file_name() {
                if let Some(f) = files_by_folder.get_mut(
                    &network_folder
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                ) {
                    f.push(BinaryVersion::from_filename_network(
                        filename.to_str().unwrap(),
                        network_folder.file_name().unwrap().to_str().unwrap(),
                    )?);
                } else {
                    files_by_folder.insert(
                        network_folder
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        vec![BinaryVersion::from_filename_network(
                            filename.to_str().unwrap(),
                            network_folder.file_name().unwrap().to_str().unwrap(),
                        )?],
                    );
                }
            }
        }
    }

    Ok(files_by_folder)
}

pub(crate) fn installed_binaries_file() -> Result<PathBuf, Error> {
    let mut path = config_folder_or_create()?;
    path.push(INSTALLED_BINARIES_FILENAME);
    if !path.exists() {
        InstalledBinaries::create_file(&path)?;
    }

    Ok(path)
}

pub(crate) fn initialize() -> Result<(), Error> {
    config_folder_or_create()?;
    binaries_folder()?;
    release_archive_folder()?;
    default_bin_folder()?;
    default_file_path()?;
    installed_binaries_file()?;

    Ok(())
}
