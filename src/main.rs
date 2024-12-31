use clap::Parser;
use handle_commands::initialize;
use handle_commands::{
    handle_component, handle_default, handle_override, handle_show, handle_update, handle_which,
    print_completion_instructions,
};

use anyhow::anyhow;
use anyhow::Error;

mod commands;
mod handle_commands;
mod types;
use commands::{Commands, ComponentCommands, Suiup};

use std::env;
use std::path::PathBuf;
use clap::CommandFactory;
use std::io;
use crate::commands::Shell;

const GITHUB_REPO: &str = "MystenLabs/sui";
const RELEASES_ARCHIVES_FOLDER: &str = "releases";

fn get_data_home() -> PathBuf {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os("USERPROFILE").expect("USERPROFILE not set"));
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
                let mut home = PathBuf::from(env::var_os("USERPROFILE").expect("USERPROFILE not set"));
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
        env::var_os("TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
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
        let mut path = PathBuf::from(
            env::var_os("LOCALAPPDATA")
                .expect("LOCALAPPDATA not set"),
        );
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
            name,
            network_release,
            version,
            nightly,
        } => {
            handle_component(ComponentCommands::Add {
                name,
                network_release: network_release.unwrap_or_else(|| "testnet".to_string()),
                version,
                debug: false,
                nightly,
            })
            .await?
        }
        Commands::Show => handle_show()?,
        Commands::Update { name } => handle_update(name).await?,
        Commands::Which => handle_which()?,
        Commands::Completion { shell } => {
            let mut cmd = Suiup::command();
            let shell_type = match shell {
                Shell::Bash => clap_complete::Shell::Bash,
                Shell::Fish => clap_complete::Shell::Fish,
                Shell::Zsh => clap_complete::Shell::Zsh,
            };
            clap_complete::generate(shell_type, &mut cmd, "suiup", &mut io::stdout());
            print_completion_instructions(&shell);
        }
    }
    Ok(())
}
