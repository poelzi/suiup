use clap::{Parser, Subcommand, ValueEnum};
use flate2::read::GzDecoder;
use reqwest::Client;
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter},
    fs::{self, set_permissions, File},
    io::{BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
};
use tar::Archive;
use trauma::{download::Download, downloader::DownloaderBuilder};

use anyhow::anyhow;
use anyhow::Error;

use serde::{Deserialize, Serialize};

const GITHUB_REPO: &str = "mystenlabs/sui";
const RELEASES_ARCHIVES_FOLDER: &str = "releases";
const SUIUP_FOLDER: &str = ".suiup";
const BINARIES_FOLDER: &str = "binaries";
const DEFAULT_BIN_FOLDER: &str = "default-bin";
const DEFAULT_VERSION_FILENAME: &str = "default_version.json";

struct Binaries {
    binaries: Vec<BinaryVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BinaryVersion {
    binary_name: String,
    network: Network,
    version: String,
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

#[derive(
    Copy, Deserialize, Serialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
enum Network {
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

#[derive(Parser)]
#[command(name = "suiup")]
#[command(about = "Sui CLI version manager.")]
struct Suiup {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand)]
    Component(ComponentCommands),
    #[command(subcommand)]
    Default(DefaultCommands),
    #[command(about = "Show installed and active Sui binaries")]
    Show,
    #[command(about = "Update binary")]
    Update,
    #[command(about = "Override project’s CLI version")]
    Override,
    #[command(about = "Show the path of the active CLI binary")]
    Which,
}

#[derive(Subcommand)]
enum ComponentCommands {
    #[command(about = "List available components")]
    List,
    #[command(about = "Add one or more components")]
    Add {
        name: Vec<String>,
        #[arg(long, value_enum, default_value_t = Network::Testnet)]
        network: Network,
        #[arg(
            long,
            help = "Version of the component to install. If not provided, the latest version will be installed."
        )]
        version: Option<String>,
    },
    #[command(about = "Remove one or more component")]
    Remove { name: Vec<String> },
}

#[derive(Subcommand)]
enum DefaultCommands {
    #[command(about = "Get the default Sui CLI version")]
    Get,
    #[command(about = "Set the default Sui CLI version")]
    Set {
        // #[arg(
        // long,
        // help = "Component(s) to be set as default. Must be provided, together with the network. If no version is provided, the latest version available locally will be set."
        // )]
        /// Component(s) to be set as default. Must be provided, together with the network. If no
        /// version is provided, the latest version available locally will be set.
        name: Vec<String>,
        #[arg(short, long, value_enum, default_value_t = Network::Testnet)]
        network: Network,
        #[arg(short, long, help = "Version of the component to set to default.")]
        /// Version of the component to set to default.
        version: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize, Debug, Clone)]
struct Asset {
    browser_download_url: String,
    name: String,
    size: u64,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Suiup::parse();

    match args.command {
        Commands::Component(cmd) => handle_component(cmd).await.map_err(|e| anyhow!("{e}"))?,
        Commands::Default(cmd) => handle_default(cmd)?,
        Commands::Show => handle_show()?,
        Commands::Update => handle_update(),
        Commands::Override => handle_override(),
        Commands::Which => handle_which()?,
    }
    Ok(())
}

fn available_components() -> &'static [&'static str] {
    &["sui", "sui-bridge", "sui-faucet"]
}

async fn handle_component(cmd: ComponentCommands) -> Result<(), Error> {
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
            network,
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
                download_release(&network.to_string())
                    .await
                    .map_err(|_| anyhow!("Could not download latest release"))?
            };
            let version = extract_version_from_release(&filename)?;
            for binary in &name {
                if !check_if_binaries_exist(binary, &network, &version)? {
                    println!("Adding component: {version}-{binary}");
                    extract_component(binary, network, &filename)
                        .map_err(|_| anyhow!("Could not extract component"))?;
                } else {
                    println!("Component {binary}-{version} already installed");
                }
            }
            update_after_install(&name, network, &version)?;
        }
        ComponentCommands::Remove { name } => {
            // println!("Removing component: {}", name);
            // Implement removing the component here
        }
    }
    Ok(())
}

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

async fn find_last_release_by_network(releases: Vec<Release>, network: &str) -> Option<Release> {
    releases
        .into_iter()
        .find(|r| r.assets.iter().any(|a| a.name.contains(network)))
}

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

async fn download_release(network: &str) -> Result<String, anyhow::Error> {
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

fn extract_version_from_release(release: &str) -> Result<String, Error> {
    let re = regex::Regex::new(r"v\d+\.\d+\.\d+").unwrap();
    let captures = re
        .captures(release)
        .ok_or_else(|| anyhow!("Could not extract version from release"))?;

    Ok(captures.get(0).unwrap().as_str().to_string())
}

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

fn default_bin_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = config_folder_or_create()?;
    path.push(DEFAULT_BIN_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

fn release_archive_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = config_folder_or_create()?;
    path.push(RELEASES_ARCHIVES_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

fn binaries_folder() -> Result<PathBuf, anyhow::Error> {
    let mut path = config_folder_or_create()?;
    path.push(BINARIES_FOLDER);
    if !path.is_dir() {
        std::fs::create_dir_all(path.as_path())
            .map_err(|e| anyhow!("Could not create directory {} due to {e}", path.display()))?;
    }
    Ok(path)
}

fn handle_default(cmd: DefaultCommands) -> Result<(), Error> {
    match cmd {
        DefaultCommands::Get => {
            let default = std::fs::read_to_string(default_file_path()?)?;
            let default: HashMap<String, (Network, Version)> = serde_json::from_str(&default)?;
            let default_binaries = Binaries::from(default);
            println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");
        }

        DefaultCommands::Set {
            name,
            network,
            version,
        } => {
            // a map of network --> to binary filenames
            let installed_binaries = installed_binaries_grouped_by_network()?;
            let binaries = installed_binaries.get(&network.to_string());
            if let Some(binaries) = binaries {
                for binary in &name {
                    if !binaries.contains(&format!("{}-{}", &binary, version)) {
                        println!(
                            "Component {binary}-{version} from {network} release is not installed. Use suiup show to see installed components."
                        );
                        return Ok(());
                    }
                }
            } else {
                println!("No components installed from {network} release");
                return Ok(());
            }
            update_default_version_file(&name, network, &version)?
        }
    }

    Ok(())
}

fn update_after_install(name: &Vec<String>, network: Network, version: &str) -> Result<(), Error> {
    let prompt = "Do you want to set this new installed version as the default one? (y/n) ";

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

type Version = String;

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

fn handle_show() -> Result<(), Error> {
    let default = std::fs::read_to_string(default_file_path()?)?;
    let default: HashMap<String, (Network, Version)> = serde_json::from_str(&default)?;
    let default_binaries = Binaries::from(default);

    println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");

    let installed_binaries = installed_binaries_grouped_by_network()?;

    println!("\x1b[1mInstalled binaries:\x1b[0m");

    for (network, binaries) in installed_binaries {
        println!("[{network}]");
        for binary in binaries {
            println!("    {binary}");
        }
    }

    Ok(())
}

fn handle_update() {
    println!("Updating components...");
    // Implement the update logic here
}

fn handle_override() {
    println!("Overriding project’s CLI version...");
    // Implement the override logic here
}

fn handle_which() -> Result<(), Error> {
    let default_bin = default_bin_folder()?;
    println!("Active CLI binary: {}", default_bin.display());
    // Implement displaying the path to the active CLI binary here
    Ok(())
}

fn default_file_path() -> Result<PathBuf, Error> {
    let mut path = config_folder_or_create()?;
    path.push(DEFAULT_VERSION_FILENAME);
    if !path.exists() {
        let mut file = File::create(&path)?;
        let default = HashMap::<String, (String, String)>::new();
        let default_str = serde_json::to_string(&default)?;
        println!("Creating default version file: {}", default_str);
        file.write_all(serde_json::to_string(&default)?.as_bytes())?;
    }
    Ok(path)
}

fn installed_binaries_grouped_by_network() -> Result<HashMap<String, Vec<String>>, Error> {
    let mut files_by_folder: HashMap<String, Vec<String>> = HashMap::new();
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

    for folder in &folders {
        let paths = fs::read_dir(folder)?
            .filter_map(Result::ok) // Filter out errors, if any
            .filter(|entry| entry.path().is_file())
            .collect::<Vec<_>>();

        for p in paths {
            if let Some(filename) = p.path().file_name() {
                if let Some(f) = files_by_folder
                    .get_mut(&folder.file_name().unwrap().to_string_lossy().to_string())
                {
                    f.push(filename.to_string_lossy().to_string());
                } else {
                    files_by_folder.insert(
                        folder.file_name().unwrap().to_string_lossy().to_string(),
                        vec![filename.to_string_lossy().to_string()],
                    );
                }
            }
        }
    }

    Ok(files_by_folder)
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

impl FromStr for Network {
    type Err = ();

    fn from_str(input: &str) -> Result<Network, Self::Err> {
        match input.to_lowercase().as_str() {
            "devnet" => Ok(Network::Devnet),
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            _ => Err(()),
        }
    }
}
