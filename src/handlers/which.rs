use crate::paths::get_default_bin_dir;
use anyhow::Error;

/// Handles the `which` command
pub fn handle_which() -> Result<(), Error> {
    let default_bin = get_default_bin_dir();
    println!("Default binaries path: {}", default_bin.display());
    // Implement displaying the path to the active CLI binary here
    Ok(())
}
