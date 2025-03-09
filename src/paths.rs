// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::path::PathBuf;

#[cfg(not(windows))]
const XDG_DATA_HOME: &str = "XDG_DATA_HOME";
#[cfg(not(windows))]
const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
#[cfg(not(windows))]
const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
#[cfg(not(windows))]
const HOME: &str = "HOME";

pub fn get_data_home() -> PathBuf {
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
        env::var_os(XDG_DATA_HOME)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os(HOME).expect("HOME not set"));
                home.push(".local");
                home.push("share");
                home
            })
    }
}

pub fn get_config_home() -> PathBuf {
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
        env::var_os(XDG_CONFIG_HOME)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os("HOME").expect("HOME not set"));
                home.push(".config");
                home
            })
    }
}

pub fn get_cache_home() -> PathBuf {
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
        env::var_os(XDG_CACHE_HOME)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = PathBuf::from(env::var_os("HOME").expect("HOME not set"));
                home.push(".cache");
                home
            })
    }
}

pub fn get_suiup_data_dir() -> PathBuf {
    let mut path = get_data_home();
    path.push("suiup");
    path
}

pub fn get_suiup_config_dir() -> PathBuf {
    let mut path = get_config_home();
    path.push("suiup");
    path
}

pub fn get_suiup_cache_dir() -> PathBuf {
    let mut path = get_cache_home();
    path.push("suiup");
    path
}

pub fn get_default_bin_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let mut path = PathBuf::from(env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA not set"));
        path.push("bin");
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
        }
        path
    }

    #[cfg(not(windows))]
    {
        let mut path = PathBuf::from(env::var_os(HOME).expect("HOME not set"));
        path.push(".local");
        path.push("bin");
        path
    }
}

pub fn get_config_file(name: &str) -> PathBuf {
    let mut path = get_suiup_config_dir();
    path.push(name);
    path
}
