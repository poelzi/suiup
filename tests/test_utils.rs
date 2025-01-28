use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestEnv {
    pub temp_dir: TempDir,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub bin_dir: PathBuf,
    original_env: Vec<(String, String)>,
}

impl TestEnv {
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let base = temp_dir.path();

        let data_dir = base.join("data");
        let config_dir = base.join("config");
        let cache_dir = base.join("cache");
        let bin_dir = base.join("bin");

        // Create directories
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::create_dir_all(&bin_dir)?;

        // Store original env vars
        let vars_to_capture = vec![
            "HOME",
            "XDG_DATA_HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "PATH",
        ];

        let original_env = vars_to_capture
            .into_iter()
            .filter_map(|var| env::var(var).ok().map(|val| (var.to_string(), val)))
            .collect();

        // Set test env vars
        env::set_var("XDG_DATA_HOME", &data_dir);
        env::set_var("XDG_CONFIG_HOME", &config_dir);
        env::set_var("XDG_CACHE_HOME", &cache_dir);

        // Add bin dir to PATH
        let path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.display(), path);
        env::set_var("PATH", new_path);

        Ok(Self {
            temp_dir,
            data_dir,
            config_dir,
            cache_dir,
            bin_dir,
            original_env,
        })
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Restore original env vars
        for (var, val) in &self.original_env {
            env::set_var(var, val);
        }
    }
}

// Mock HTTP client for testing
#[cfg(test)]
pub mod mock_http {
    use mockall::mock;
    use reqwest::Response;

    mock! {
        pub HttpClient {
            async fn get(&self, url: String) -> reqwest::Result<Response>;
        }
    }
}

