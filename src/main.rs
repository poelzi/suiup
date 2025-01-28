use clap::Parser;
use handle_commands::initialize;
use handle_commands::{handle_component, handle_default, handle_show, handle_update, handle_which};

use anyhow::anyhow;
use anyhow::Error;

mod commands;
mod handle_commands;
mod mvr;
mod types;
mod walrus;
use commands::{Commands, ComponentCommands, Suiup};

use clap::CommandFactory;
use std::env;
use std::path::PathBuf;

const GITHUB_REPO: &str = "MystenLabs/sui";
const RELEASES_ARCHIVES_FOLDER: &str = "releases";

fn get_data_home() -> PathBuf {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home =
                    PathBuf::from(env::var_os("USERPROFILE").expect("USERPROFILE not set"));
                home.push("AppData");
                home.push("Local");
                home
            })
    }

    #[cfg(not(windows))]
    {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os("HOME").expect("HOME not set"));
                home.push(".local");
                home.push("share");
                home
            })
    }
}

fn get_config_home() -> PathBuf {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home =
                    PathBuf::from(env::var_os("USERPROFILE").expect("USERPROFILE not set"));
                home.push("AppData");
                home.push("Local");
                home
            })
    }

    #[cfg(not(windows))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os("HOME").expect("HOME not set"));
                home.push(".config");
                home
            })
    }
}

fn get_cache_home() -> PathBuf {
    #[cfg(windows)]
    {
        env::var_os("TEMP").map(PathBuf::from).unwrap_or_else(|| {
            let mut home = PathBuf::from(env::var_os("USERPROFILE").expect("USERPROFILE not set"));
            home.push("AppData");
            home.push("Local");
            home.push("Temp");
            home
        })
    }

    #[cfg(not(windows))]
    {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os("HOME").expect("HOME not set"));
                home.push(".cache");
                home
            })
    }
}

fn get_suiup_data_dir() -> PathBuf {
    let mut path = get_data_home();
    path.push("suiup");
    path
}

fn get_suiup_config_dir() -> PathBuf {
    let mut path = get_config_home();
    path.push("suiup");
    path
}

fn get_suiup_cache_dir() -> PathBuf {
    let mut path = get_cache_home();
    path.push("suiup");
    path
}

fn get_default_bin_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let mut path = PathBuf::from(env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA not set"));
        path.push(".local");
        path.push("bin");
        path
    }

    #[cfg(not(windows))]
    {
        let mut path = PathBuf::from(env::var_os("HOME").expect("HOME not set"));
        path.push(".local");
        path.push("bin");
        path
    }
}

fn get_config_file(name: &str) -> PathBuf {
    let mut path = get_suiup_config_dir();
    path.push(name);
    path
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    initialize()?;
    let args = Suiup::parse();

    match args.command {
        Commands::Component(cmd) => handle_component(cmd).await.map_err(|e| anyhow!("{e}"))?,
        Commands::Default(cmd) => handle_default(cmd)?,
        Commands::Install {
            components,
            nightly,
            yes,
        } => {
            handle_component(ComponentCommands::Add {
                components,
                debug: false,
                nightly,
                yes,
            })
            .await?
        }
        Commands::Show => handle_show()?,
        Commands::Update { name } => handle_update(name).await?,
        Commands::Which => handle_which()?,
        Commands::Completion { shell } => {
            let mut cmd = Suiup::command();
            // Generate to string first to validate the output
            let mut buf = Vec::new();
            clap_complete::generate(shell, &mut cmd, "suiup", &mut buf);

            // Print to stdout if generation was successful
            if let Ok(s) = String::from_utf8(buf) {
                print!("{}", s);
            }
            // print_completion_instructions(&shell);
        }
    }
    Ok(())
}
