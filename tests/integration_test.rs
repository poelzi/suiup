mod test_utils;

#[cfg(test)]
mod tests {
    use crate::test_utils::TestEnv;
    use anyhow::Result;
    use assert_cmd::Command;
    use predicates::prelude::*;

    #[tokio::test]
    async fn test_install_and_use_binary() -> Result<()> {
        let test_env = TestEnv::new()?;
        test_env.copy_testnet_releases_to_cache()?;

        // Run install command
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("install")
            .arg("sui")
            .arg("testnet-v1.39.3")
            .arg("-y")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("'sui' extracted successfully!"));

        // Verify binary exists in correct location
        let binary_path = test_env.data_dir.join("suiup/binaries/testnet/sui-v1.39.3");
        assert!(binary_path.exists());

        // Verify default binary exists
        let default_sui_binary = test_env.bin_dir.join("sui");
        assert!(default_sui_binary.exists());

        // Test binary execution
        let mut cmd = Command::new(default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.39.3"));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_workflow() -> Result<()> {
        let test_env = TestEnv::new()?;

        // Install older version
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("install")
            .arg("mvr")
            .arg("v0.0.4")
            .arg("-y")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());

        cmd.assert().success();

        // Run update
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("update")
            .arg("mvr")
            .arg("-y")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());

        cmd.assert().success();

        // Verify new version exists
        let binary_path = test_env.data_dir.join("suiup/binaries/standalone");
        let folders = std::fs::read_dir(&binary_path)?;
        let num_files: Vec<_> = folders.into_iter().collect();
        // should have at least 2 versions, 1.39.0 and whatever latest is
        assert!(num_files.len() >= 1);

        // Verify default binary exists
        let default_sui_binary = test_env.bin_dir.join("mvr");
        assert!(default_sui_binary.exists());

        // Test binary execution
        let mut cmd = Command::new(default_sui_binary);
        cmd.arg("--version");
        cmd.assert().success();

        Ok(())
    }

    #[tokio::test]
    async fn test_default_workflow() -> Result<(), anyhow::Error> {
        let test_env = TestEnv::new()?;
        test_env.copy_testnet_releases_to_cache()?;

        // Install 1.39.3
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("install")
            .arg("sui")
            .arg("testnet-v1.39.3")
            .arg("-y")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("'sui' extracted successfully!"));
        // Test binary execution
        let default_sui_binary = test_env.bin_dir.join("sui");
        let mut cmd = Command::new(&default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.39.3"));
        // Install 1.40.1
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("install")
            .arg("sui")
            .arg("testnet-v1.40.1")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("'sui' extracted successfully!"));
        // Test binary execution
        let default_sui_binary = test_env.bin_dir.join("sui");
        let mut cmd = Command::new(&default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.39.3"));

        // Switch from 1.39.3 to 1.40.1
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("default")
            .arg("set")
            .arg("sui")
            .arg("testnet-v1.40.1")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());

        cmd.assert().success();

        let mut cmd = Command::new(&default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.40.1"));

        // Switch back from 1.40.1 to 1.39.3
        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("default")
            .arg("set")
            .arg("sui")
            .arg("testnet-v1.39.3")
            .env("XDG_DATA_HOME", &test_env.data_dir)
            .env("XDG_CONFIG_HOME", &test_env.config_dir)
            .env("XDG_CACHE_HOME", &test_env.cache_dir)
            .env("HOME", &test_env.temp_dir.path());

        cmd.assert().success();

        let mut cmd = Command::new(default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.39.3"));

        Ok(())
    }
}
