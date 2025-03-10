[!WARNING] Highly experimental, use at your own risk. Not recommended for production use. No warranty is provided.

# Overview
`suiup` is a tool to install and manage different versions of Sui CLI tools. It allows you to easily switch between different versions of the Sui CLI tools, such as `sui`, `mvr`, and `walrus`.

Versions are specified in the format `network` and optionally a version (`-vX.Y.Z`) for `sui` (e.g., `testnet-v1.40.0`, `devnet`, `mainnet`) and `vX.Y.Z` for `mvr` (e.g., `v0.0.7`).

Please note that due to the different release mechanisms between these different tools, `suiup` does its best to make it simple to install them. 

# Installation


**From Cargo**
```bash
cargo install https://github.com/Mystenlabs/suiup.git --locked
```

**From Release**

1. Download the latest release from [Releases](https://github.com/Mystenlabs/suiup/releases).
2. Unzip the downloaded file.
3. Add the `suiup` binary to folder that is on your `PATH` environment variable, or add that folder to the `PATH`.

# Quick Start

**Install `sui` -- this will install the latest available testnet release**
```bash
suiup install sui
```

**Install `sui` with specific version**
```bash
suiup install sui testnet-v1.40.0
suiup install sui devnet # this will install the latest available devnet release
```

**Update `sui` to latest version**
This will update the `sui` binary to the latest available release for the current default binary. For example, if binary is a devnet release, it will update to the latest devnet release.
```bash
suiup update sui
```

**Install `mvr` -- this will install the latest available release of Move Registry CLI**
```bash
suiup install mvr
```

**List available binaries to install**
```bash
suiup list
```

**Show installed versions**
```bash
suiup show
```

**Switch between versions. Note that `default set` requires to specify a version!**
```bash
suiup default get
suiup default set sui testnet-v1.40.0
suiup default set mvr v0.0.7
suiup default set sui testnet-v1.40.0 --debug # set the default version to be the sui-debug binary
```

**Show where the default binaries are installed**
```bash
suiup which
```

# Advanced Usage

### Pre-requisites
- [Rust](https://www.rust-lang.org/tools/install) (if you want to install from branch)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (if you want to install from branch)
- [Git](https://git-scm.com/downloads) (if you want to install from branch)

Installing a nightly version is highly experimental and might not work as expected. Avoid using it unless you really need to.

[!NOTE] 
`--nightly` will replace the current nightly binary, if any. Currently, there's no support for multiple nightly versions. Hope to add it in the future!

**Install from branch (requires cargo + rust installed!)**
```bash
suiup install mvr --nightly # installs from main if branch name is omitted
suiup install mvr --nightly my_branch
```
[!INFO]
There is a `--debug` flag that can be used in two ways:
- for `sui` binary, it will install the `sui-debug` binary from the release archive which contains debug symbols and it's required to run `sui move test --coverage`.
- for when using `--nightly`, it will build the binary from source with debug symbols. By default, `--nightly` builds in release mode as per `cargo install`'s defaults.

**Install MVR from nightly in debug mode**
```bash
suiup install mvr --nightly --debug
```

**Switch default versions**
```bash
suiup default set sui --nightly
```

**Using it in CI**
As the tool requires to download releases and files from GitHub, it is recommended to use a GitHub token to avoid rate limits. You can set the `GITHUB_TOKEN` environment variable to your GitHub token or pass in the `--github-token` argument.

```bash
GITHUB_TOKEN=your_github_token suiup install sui
```
