#!/bin/sh
# install.sh - Pre-compiled binary installer for LGTUI (Linux Gaming Terminal UI)
# Installs LGTUI without compiling from source.

set -e

COLOR_GREEN='\033[0;32m'
COLOR_BLUE='\033[0;34m'
COLOR_YELLOW='\033[1;33m'
COLOR_RED='\033[0;31m'
COLOR_RESET='\033[0m'

echo "=== LGTUI Binary Installer ==="

# Check target OS
if [ "$(uname)" != "Linux" ]; then
    echo "${COLOR_RED}[ERROR] LGTUI is only supported on Linux.${COLOR_RESET}"
    exit 1
fi

# Detect architecture
ARCH="$(uname -m)"
if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "amd64" ]; then
    echo "${COLOR_RED}[ERROR] LGTUI binary releases are only compiled for x86_64 architectures.${COLOR_RESET}"
    exit 1
fi

# Determine install destination
INSTALL_DIR=""
USE_SUDO=0

if [ "$(id -u)" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
else
    # Non-root installation path
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    
    # Check if destination is in PATH
    case ":$PATH:" in
        *:"$INSTALL_DIR":*) ;;
        *)
            echo "${COLOR_YELLOW}[WARNING] $INSTALL_DIR is not in your PATH environment variable.${COLOR_RESET}"
            echo "${COLOR_YELLOW}[WARNING] You may need to add it to your shell config (.bashrc / .zshrc):${COLOR_RESET}"
            echo "  export PATH=\"\$PATH:\$HOME/.local/bin\""
            ;;
    esac
fi

echo "Installing to: $INSTALL_DIR/lgtui"

# Fetch latest release info
echo "Fetching latest release details..."
REPO="ucmz851/lgtui"

# Query the GitHub API for the latest release tag, with fallback
LATEST_TAG=""
if command -v curl >/dev/null 2>&1; then
    LATEST_TAG=$(curl -sSL --connect-timeout 4 "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
fi

if [ -z "$LATEST_TAG" ] || [ "$LATEST_TAG" = "null" ]; then
    LATEST_TAG="v0.1.0"
fi

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/lgtui-linux-x86_64.tar.gz"

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Determine if running from the local repository directory or via curl
if [ -f Cargo.toml ] && { [ -f "target/debug/lgtui" ] || [ -f "target/release/lgtui" ] || command -v cargo >/dev/null 2>&1; }; then
    echo "Detected local repository clone. Installing local binary..."
    if [ -f "target/release/lgtui" ]; then
        cp "target/release/lgtui" "$TMP_DIR/lgtui"
    elif [ -f "target/debug/lgtui" ]; then
        cp "target/debug/lgtui" "$TMP_DIR/lgtui"
    else
        echo "No target found. Compiling release binary..."
        cargo build --release
        cp target/release/lgtui "$TMP_DIR/lgtui"
    fi
else
    echo "Downloading LGTUI release ($LATEST_TAG) from GitHub..."
    if command -v curl >/dev/null 2>&1; then
        curl -sSL -o "$TMP_DIR/lgtui.tar.gz" "$DOWNLOAD_URL"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$TMP_DIR/lgtui.tar.gz" "$DOWNLOAD_URL"
    else
        echo "${COLOR_RED}[ERROR] Neither curl nor wget was found. Cannot download release.${COLOR_RESET}"
        exit 1
    fi
    tar -xzf "$TMP_DIR/lgtui.tar.gz" -C "$TMP_DIR"
fi

# Copy binary to destination
cp "$TMP_DIR/lgtui" "$INSTALL_DIR/lgtui"
chmod +x "$INSTALL_DIR/lgtui"

# Setup Desktop Entry file
echo "Installing desktop application entry..."
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR"

cat <<EOF > "$DESKTOP_DIR/lgtui.desktop"
[Desktop Entry]
Name=LGTUI
Comment=Linux Gaming Terminal UI
Exec=$INSTALL_DIR/lgtui
Icon=utilities-terminal
Terminal=true
Type=Application
Categories=Game;Utility;
Keywords=wine;gaming;tui;proton;
EOF

chmod +x "$DESKTOP_DIR/lgtui.desktop"

echo "${COLOR_GREEN}=== Installation Completed Successfully! ===${COLOR_RESET}"
echo "You can now run LGTUI by typing ${COLOR_BLUE}lgtui${COLOR_RESET} in your terminal,"
echo "or launch it from your desktop application menu."
