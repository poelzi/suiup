> [!CAUTION] 
> Highly experimental, use at your own risk. Not recommended for production use. No warranty is provided. Might scrap it and rewrite it later!!!

# Overview
`suiup` is a tool to install and manage different versions of CLI tools for working in the Sui ecosystem. It allows you to easily install and switch between different versions of `sui`, `mvr`, and limited support for `walrus`.

Versions are specified in the format `network` and optionally a version (`-vX.Y.Z`) for `sui` (e.g., `testnet-v1.40.0`, `devnet`, `mainnet`) and `vX.Y.Z` for `mvr` (e.g., `v0.0.7`).
`walrus` CLI is poorly supported at the time of writing.

Please note that due to the different release mechanisms between these different tools, `suiup` does its best to make it simple to install them, but please read the examples below to understand how it works.

# Supported OS

| OS       | Architecture      | Status         |
|----------|-------------------|----------------|
| Linux    | x86_64 (amd64)    | ✅ Supported   |
| Linux    | aarch64 (ARM64)   | ✅ Supported   |
| macOS    | x86_64 (amd64)    | ✅ Supported   |
| macOS    | aarch64 (ARM64)   | ✅ Supported   |
| Windows  | x86_64 (amd64)    | Limited support (might or might not work) |
| Windows  | aarch64 (ARM64)   | Limited support (might or might not work) |

# Installation

### From Script
```bash
curl -sSfL https://raw.githubusercontent.com/Mystenlabs/suiup/main/install.sh | sh
```

### From Cargo
```bash
cargo install https://github.com/Mystenlabs/suiup.git --locked
```

### From Release

1. Download the latest release from [Releases](https://github.com/Mystenlabs/suiup/releases).
2. Unzip the downloaded file.
3. Add the `suiup` binary to folder that is on your `PATH` environment variable, or add that folder to the `PATH`.
4. (Optional) restart your terminal if you made changes to your `PATH`.

# Quick Start

> [!TIP]
> Pass the `--yes` flag to skip confirmation prompts, thus accepting to updating the default binary to the one you are installing.

### Install `sui` -- this will install the latest available testnet release
```bash
suiup install sui
```

### Install `sui` with specific network (and version)
```bash
suiup install sui devnet # this will install the latest available devnet release
suiup install sui testnet-v1.40.1 # this will install the testnet v1.40.1 release
```

### Update `sui` to latest version
This will check for newer releases of those that are already installed, and tries to download the new ones. Recommended to specify which release to update.
```bash
suiup update sui devnet # recommended
suiup update sui
```

### Install `mvr` (Move Registry CLI)
```bash
suiup install mvr
suiup install mvr v0.0.8 # this will install the MVR CLI v0.0.8 release
```

### List available binaries to install
```bash
suiup list
```

### Show installed versions
```bash
suiup show
```

### Switch between versions. Note that `default set` requires to specify a version!
```bash
suiup default get
suiup default set sui testnet-v1.40.0
suiup default set mvr v0.0.7
suiup default set sui testnet-v1.40.0 --debug # set the default version to be the sui-debug binary
```

### Show where the default binaries are installed
```bash
suiup which
```

# Advanced Usage

### Pre-requisites
- [Rust](https://www.rust-lang.org/tools/install) (if you want to install from branch)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (if you want to install from branch)
- [Git](https://git-scm.com/downloads) (if you want to install from branch)

Installing a nightly version is highly experimental and might not work as expected. Avoid using it unless you really need to.

> [!IMPORTANT]
> `--nightly` will replace the current nightly binary, if any. Currently, there's no support for multiple nightly versions. Hope to add it in the future!

### Install from branch (requires cargo + rust installed!)
```bash
suiup install mvr --nightly # installs from main if branch name is omitted
suiup install mvr --nightly my_branch
```
> [!NOTE]
> There is a `--debug` flag that can be used in two ways:
> - for `sui` binary, it will install the `sui-debug` binary from the release archive which contains debug symbols and it's required to run `sui move test --coverage`.
> - for when using `--nightly`, it will build the binary from source with debug symbols. By default, `--nightly` builds in release mode as per `cargo install`'s defaults.

### Install MVR from nightly in debug mode
```bash
suiup install mvr --nightly --debug
```

### Switch default versions
```bash
suiup default set sui --nightly
```

### Using it in CI
As the tool requires to download releases and files from GitHub, it is recommended to use a GitHub token to avoid rate limits. You can set the `GITHUB_TOKEN` environment variable to your GitHub token or pass in the `--github-token` argument.

In the CI environment, you can set the `GITHUB_TOKEN` environment variable to your GitHub token, then you can run the `suiup` command as usual:
```bash
env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

steps:
- name: Install sui
  run: |
    suiup install sui
```

Or if you're calling from shell:
```bash
GITHUB_TOKEN=your_github_token suiup install sui
```

## Paths used by the `suiup` tool

**[Unix/MacOS]**
The tool uses these environment variables to store data.
- `XDG_DATA_HOME`
- `XDG_CACHE_HOME`
- `XDG_CONFIG_HOME`
- `HOME/.local/bin` for storing default binaries to be used. Make sure this is on your `PATH` or set up `SUIUP_DEFAULT_BIN_DIR` env variable to point to a different directory.

**[Windows]**
- `LOCALAPPDATA` or `USERPROFILE\AppData\Local` for storing data
- `TEMP` or `USERPROFILE\AppData\Local\Temp` for caching
- `LOCALAPPDATA\bin` for storing default binaries to be used

## Known issues
- `suiup install mvr --nightly` might fail on **Windows** because of issues with compiling the `mvr-cli` crate from the repository. Just install the latest release instead.
- `suiup remove` does not work well. Do not use it.
- `suiup install walrus` will always install the latest binary. Need some work to properly support Walrus CLI!

## Troubleshooting

### Where are the default binaries copied to?

For Unix/MacOS they are copied to `$HOME/.local/bin` (or where your `SUIUP_DEFAULT_BIN_DIR` env var points to) and for Windows they are copied to `LOCALAPPDATA\bin`. Make sure you have these folders on the `PATH`.
