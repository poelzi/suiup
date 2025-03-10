> [!WARNING] Highly experimental, use at your own risk. Not recommended for production use. No warranty is provided.

# Overview
`suiup` is a tool to install and manage different versions of Sui CLI tools. It allows you to easily switch between different versions of the Sui CLI tools, such as `sui`, `mvr`, and `walrus`.

Please note that due to the different release mechanisms between these different tools, `suiup` does its best to make it simple to install them. However, for example, `mvr` does not have a release tied to a specific network, so 
you will only need to specify the version without the release name (e.g., `suiup install mvr v0.0.7` vs `suiup install sui testnet-v1.40.0`) in the case of `sui`.

Versions are specified in the format `network` and optionally a version (`-vX.Y.Z`) for `sui` (e.g., `testnet-v1.40.0`, `devnet`, `mainnet`) and `vX.Y.Z` for `mvr` (e.g., `v0.0.7`).

# Installation

## Pre-requisites
- [Rust](https://www.rust-lang.org/tools/install) (if you want to install from branch)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (if you want to install from branch)
- [Git](https://git-scm.com/downloads) (if you want to install from branch)

**From Cargo**
```bash
cargo install https://github.com/Mystenlabs/suiup.git --locked
```

**From Release**

1. Download the latest release from [Releases](https://github.com/Mystenlabs/suiup/releases).
2. Unzip the downloaded file.
3. Add the `suiup` binary to folder that is on your `PATH` environment variable, or add that folder to the `PATH`.

# Quick Start

**Install `sui` -- this will install the latest known testnet release**
```bash
suiup install sui
```

**Install `sui` with specific version**
```bash
suiup install sui testnet-v1.40.0
suiup install sui devnet # this will install the latest known devnet release
```

**Install `mvr` -- this will install the last release of Move Registry CLI**
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


# Advanced Usage

**Install from branch (requires cargo + rust installed!)**
```bash
suiup install mvr --nightly # installs from main if branch name is omitted
suiup install mvr --nightly my_branch
```

There is a `--debug` flag that can be used in two ways:
- for `sui` binary, it will install the `sui-debug` binary from the release archive which contains debug symbols and it's required to run `sui move test --coverage`.
- for when using `--nightly`, it will build the binary from source with debug symbols. By default, `--nightly` builds in release mode as per `cargo install`'s defaults.

**Install MVR from nightly in debug mode**
```bash
suiup install mvr --nightly --debug
```
