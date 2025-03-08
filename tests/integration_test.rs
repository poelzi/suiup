// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod test_utils;

#[cfg(test)]
mod tests {
    use crate::test_utils::TestEnv;
    use anyhow::Result;
    use assert_cmd::Command;
    use predicates::prelude::*;

    #[cfg(not(windows))]
    const DATA_HOME: &str = "XDG_DATA_HOME";
    #[cfg(not(windows))]
    const CONFIG_HOME: &str = "XDG_CONFIG_HOME";
    #[cfg(not(windows))]
    const CACHE_HOME: &str = "XDG_CACHE_HOME";
    #[cfg(not(windows))]
    const HOME: &str = "HOME";

    #[cfg(windows)]
    const DATA_HOME: &str = "XDG_DATA_HOME";
    #[cfg(windows)]
    const CONFIG_HOME: &str = "XDG_CONFIG_HOME";
    #[cfg(windows)]
    const CACHE_HOME: &str = "XDG_CACHE_HOME";
    #[cfg(windows)]
    const HOME: &str = "HOME";

    fn suiup_command(args: Vec<&str>, test_env: &TestEnv) -> Command {
        let mut cmd = Command::cargo_bin("suiup").unwrap();
        cmd.args(args);

        cmd.env(DATA_HOME, &test_env.data_dir)
            .env(CONFIG_HOME, &test_env.config_dir)
            .env(CACHE_HOME, &test_env.cache_dir)
            .env(HOME, &test_env.temp_dir.path());
        cmd
    }

    #[tokio::test]
    async fn test_install_flags() -> Result<()> {
        let test_env = TestEnv::new()?;

        // NOT OK: nightly + version specified
        let mut cmd = suiup_command(
            vec!["install", "sui", "testnet-v1.40.1", "--nightly"],
            &test_env,
        );
        cmd.assert().failure().stderr(predicate::str::contains(
            "Error: Cannot install from nightly and a release at the same time",
        ));

        // NOT OK: !sui + debug
        let mut cmd = suiup_command(vec!["install", "mvr", "--debug"], &test_env);
        cmd.assert().failure().stderr(predicate::str::contains(
            "Error: Debug flag is only available for the `sui` binary",
        ));

        // OK: nightly + debug
        // OK: nightly (if nightly + debug work, nightly works on its own too)
        let mut cmd = suiup_command(vec!["install", "mvr", "--nightly", "--debug"], &test_env);
        cmd.assert().success();

        Ok(())
    }

    #[tokio::test]
    async fn test_install_and_use_binary() -> Result<()> {
        let test_env = TestEnv::new()?;
        test_env.copy_testnet_releases_to_cache()?;

        // Run install command
        let mut cmd = suiup_command(vec!["install", "sui", "testnet-v1.39.3", "-y"], &test_env);

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
    async fn test_install_nightly() -> Result<()> {
        Ok(())
    }

    #[tokio::test]
    async fn test_install_debug() -> Result<()> {
        let test_env = TestEnv::new()?;
        test_env.copy_testnet_releases_to_cache()?;

        // Run install command
        let mut cmd = suiup_command(vec!["install", "mvr", "--debug", "-y"], &test_env);
        cmd.assert().failure().stderr(predicate::str::contains(
            "Error: Debug flag is only available for the `sui` binary",
        ));

        // Run install command
        let mut cmd = suiup_command(
            vec!["install", "sui", "testnet-v1.39.3", "--debug", "-y"],
            &test_env,
        );
        cmd.assert().success().stdout(predicate::str::contains(
            "'sui-debug' extracted successfully!",
        ));

        // Verify binary exists in correct location
        let binary_path = test_env
            .data_dir
            .join("suiup/binaries/testnet/sui-debug-v1.39.3");
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

        let mut cmd = Command::cargo_bin("suiup")?;
        cmd.arg("default").arg("get");
        cmd.assert().success().stdout(predicate::str::contains(
            "[testnet release]
    sui-v1.39.3 (debug build)",
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_workflow() -> Result<()> {
        let test_env = TestEnv::new()?;

        // Install older version
        let mut cmd = suiup_command(vec!["install", "mvr", "v0.0.4", "-y"], &test_env);
        cmd.assert().success();

        // Run update
        let mut cmd = suiup_command(vec!["update", "mvr", "-y"], &test_env);
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
        let mut cmd = suiup_command(vec!["install", "sui", "testnet-v1.39.3", "-y"], &test_env);
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
        let mut cmd = suiup_command(vec!["install", "sui", "testnet-v1.40.1", "-y"], &test_env);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("'sui' extracted successfully!"));
        // Test binary execution
        let default_sui_binary = test_env.bin_dir.join("sui");
        let mut cmd = Command::new(&default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.40.1"));

        // Switch from 1.39.3 to 1.40.1
        let mut cmd = suiup_command(vec!["default", "set", "sui", "testnet-v1.40.1"], &test_env);
        cmd.assert().success();

        let mut cmd = Command::new(&default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.40.1"));

        // Switch back from 1.40.1 to 1.39.3
        let mut cmd = suiup_command(vec!["default", "set", "sui", "testnet-v1.39.3"], &test_env);
        cmd.assert().success();

        let mut cmd = Command::new(default_sui_binary);
        cmd.arg("--version");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.39.3"));

        Ok(())
    }
}
