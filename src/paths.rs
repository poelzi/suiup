pub fn get_binary_dir() -> PathBuf {
    let data_dir = get_data_dir();
    data_dir.join("binaries")
}

pub fn get_data_dir() -> PathBuf {
    if cfg!(windows) {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(env!("LOCALAPPDATA")))
            .join("suiup")
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(env!("HOME")).join(".local/share"))
            .join("suiup")
    }
} 