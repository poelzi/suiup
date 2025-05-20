#!/bin/bash
set -e

# Configuration
GITHUB_REPO="MystenLabs/suiup"
BINARY_NAME="suiup"
RELEASES_URL="https://github.com/${GITHUB_REPO}/releases"

# Set up colors for output
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${CYAN}suiup installer script${NC}"
echo "This script will install the suiup binary to your system."

# Get latest version from GitHub
get_latest_version() {
    # Check if GITHUB_TOKEN is set and use it for authentication
    local auth_header=""
    if [ -n "$GITHUB_TOKEN" ]; then
        auth_header="Authorization: Bearer $GITHUB_TOKEN"
    fi

    if command -v curl &>/dev/null; then
        if [ -n "$auth_header" ]; then
            curl -fsSL -H "$auth_header" "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
        else
            curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
        fi
    elif command -v wget &>/dev/null; then
        if [ -n "$auth_header" ]; then
            wget --quiet --header="$auth_header" -O- "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
        else
            wget --quiet -O- "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
        fi
    else
        echo -e "${RED}Error: Neither curl nor wget is available. Please install one of them.${NC}"
        exit 1
    fi
}

# Detect operating system
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)          echo "unknown" ;;
    esac
}

# Detect architecture
detect_arch() {
    local arch=$(uname -m)
    case "$arch" in
        x86_64|amd64) echo "x86_64" ;;
        arm64|aarch64) echo "arm64" ;;
        *)          echo "unknown" ;;
    esac
}

# Get the appropriate download URL
get_download_url() {
    local os=$1
    local arch=$2
    local version=$3
    
    # Construct the filename based on OS and architecture
    if [ "$os" = "macos" ]; then
        echo "${RELEASES_URL}/download/${version}/suiup-macOS-${arch}.tar.gz"
    elif [ "$os" = "linux" ]; then
        echo "${RELEASES_URL}/download/${version}/suiup-Linux-musl-${arch}.tar.gz"
    elif [ "$os" = "windows" ]; then
        echo "${RELEASES_URL}/download/${version}/suiup-x86_64-pc-windows-msvc"  # Adjust if needed
    else
        echo ""
    fi
}

# Determine installation directory
get_install_dir() {
    local os=$1
    
    if [ "$os" = "macos" ] || [ "$os" = "linux" ]; then
        # Use ~/.local/bin on Unix-like systems if it exists or can be created
        local local_bin="$HOME/.local/bin"
        if [ -d "$local_bin" ] || mkdir -p "$local_bin" 2>/dev/null; then
            echo "$local_bin"
        else
            # Fallback to /usr/local/bin if we can write to it
            if [ -w "/usr/local/bin" ]; then
                echo "/usr/local/bin"
            else
                # Last resort, use a directory in home
                mkdir -p "$HOME/bin"
                echo "$HOME/bin"
            fi
        fi
    elif [ "$os" = "windows" ]; then
        # On Windows, use %USERPROFILE%\.local\bin
        local win_dir="$HOME/.local/bin"
        mkdir -p "$win_dir"
        echo "$win_dir"
    else
        echo "$HOME/bin"
        mkdir -p "$HOME/bin"
    fi
}

# Check if the directory is in PATH
check_path() {
    local dir=$1
    local os=$2
    
    # Different path separators for different OSes
    local separator=":"
    if [ "$os" = "windows" ]; then
        separator=";"
    fi
    
    if [[ ":$PATH:" != *":$dir:"* ]]; then
        echo -e "${YELLOW}Warning: $dir is not in your PATH${NC}"
        
        # Provide instructions based on OS
        if [ "$os" = "macos" ] || [ "$os" = "linux" ]; then
            echo "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo -e "${GREEN}export PATH=\"$dir:\$PATH\"${NC}"
        elif [ "$os" = "windows" ]; then
            echo "Add this directory to your PATH by running this in PowerShell:"
            echo -e "${GREEN}\$env:Path += \"$separator$dir\"${NC}"
            echo "To make it permanent, add it through Windows System Properties:"
            echo "Control Panel → System → Advanced system settings → Environment Variables"
        fi
    else
        echo -e "${GREEN}$dir is already in your PATH${NC}"
    fi
}

# Download a file with curl or wget
download_file() {
    local url=$1
    local output_file=$2
    
    echo "Downloading $url to $output_file..."
    
    # Check if GITHUB_TOKEN is set and use it for authentication
    local auth_header=""
    if [ -n "$GITHUB_TOKEN" ]; then
        auth_header="Authorization: Bearer $GITHUB_TOKEN"
    fi

    if command -v curl &>/dev/null; then
        if [ -n "$auth_header" ]; then
            curl -fsSL -H "$auth_header" "$url" -o "$output_file"
        else
            curl -fsSL "$url" -o "$output_file"
        fi
    elif command -v wget &>/dev/null; then
        if [ -n "$auth_header" ]; then
            wget --quiet --header="$auth_header" "$url" -O "$output_file"
        else
            wget --quiet "$url" -O "$output_file"
        fi
    else
        echo -e "${RED}Error: Neither curl nor wget is available. Please install one of them.${NC}"
        exit 1
    fi
}

# Main installation function
install_suiup() {
    local os=$(detect_os)
    local arch=$(detect_arch)
    local version=$(get_latest_version)
    
    if [ -z "$version" ]; then
        echo -e "${RED}Error: Could not fetch latest version${NC}"
        exit 1
    fi
    
    local download_url=$(get_download_url "$os" "$arch" "$version")
    
    if [ -z "$download_url" ]; then
        echo -e "${RED}Error: Unsupported OS or architecture: $os/$arch${NC}"
        exit 1
    fi
    
    echo "Detected OS: $os"
    echo "Detected architecture: $arch"
    echo "Latest version: $version"
    echo "Download URL: $download_url"
    
    # Create temporary directory
    local tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT
    
    # Download the binary
    local binary_file="$tmp_dir/suiup.tar.gz"
    
    download_file "$download_url" "$binary_file"
    
    # Extract the binary
    echo "Extracting binary..."
    if [ "$os" = "windows" ]; then
        echo "No extraction needed for Windows binaries."
    else
        tar -xzf "$binary_file" -C "$tmp_dir"
    fi
    
    # Install to appropriate directory
    local install_dir=$(get_install_dir "$os")
    local installed_path="$install_dir/suiup"
    if [ "$os" = "windows" ]; then
        installed_path="$install_dir/suiup.exe"
    fi
    
    echo "Installing to $installed_path..."
    
    # Ensure install directory exists
    mkdir -p "$install_dir"
    
    # Move binary to install directory
    mv "$tmp_dir/suiup" "$installed_path"
    
    echo -e "${GREEN}Successfully installed suiup to $installed_path${NC}"
    
    # Check PATH
    check_path "$install_dir" "$os"
    
    echo ""
    echo -e "You can now run ${CYAN}suiup --help${NC} to get started."
    echo "For more information, visit: https://github.com/$GITHUB_REPO"
}

# Run the installer
install_suiup
