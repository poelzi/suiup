use crate::{
    handlers::installed_binaries_grouped_by_network,
    paths::default_file_path,
    types::{Binaries, Version},
};
use anyhow::Error;
use std::collections::HashMap;

/// Handles the `show` command
pub fn handle_show() -> Result<(), Error> {
    let default = std::fs::read_to_string(default_file_path()?)?;
    let default: HashMap<String, (String, Version, bool)> = serde_json::from_str(&default)?;
    let default_binaries = Binaries::from(default);

    println!("\x1b[1mDefault binaries:\x1b[0m\n{default_binaries}");

    let installed_binaries = installed_binaries_grouped_by_network(None)?;

    println!("\x1b[1mInstalled binaries:\x1b[0m");

    for (network, binaries) in installed_binaries {
        println!("[{network} release/branch]");
        for binary in binaries {
            println!("    {binary}");
        }
    }

    Ok(())
}
