#!/bin/bash
# install.sh - Pre-compiled binary installer for LGTUI

set -e

# ANSI Color Codes
COLOR_GREEN='\033[0;32m'
COLOR_BLUE='\033[0;34m'
COLOR_YELLOW='\033[1;33m'
COLOR_RED='\033[0;31m'
COLOR_CYAN='\033[0;36m'
COLOR_BOLD='\033[1m'
COLOR_RESET='\033[0m'

echo -e "${COLOR_BLUE}${COLOR_BOLD}=== LGTUI Binary Installer ===${COLOR_RESET}"

# Check target OS
if [ "$(uname)" != "Linux" ]; then
    echo -e "${COLOR_RED}[ERROR] LGTUI is only supported on Linux.${COLOR_RESET}"
    exit 1
fi

# Detect architecture
ARCH="$(uname -m)"
if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "amd64" ]; then
    echo -e "${COLOR_RED}[ERROR] LGTUI binary releases are only compiled for x86_64 architectures.${COLOR_RESET}"
    exit 1
fi

# Determine install destination
INSTALL_DIR=""
ICON_DIR=""

if [ "$(id -u)" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
    ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    
    ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
    mkdir -p "$ICON_DIR"
    mkdir -p "$HOME/.local/share/icons"
    
    case ":$PATH:" in
        *:"$INSTALL_DIR":*) ;;
        *)
            echo -e "${COLOR_YELLOW}[WARNING] $INSTALL_DIR is not in your PATH environment variable.${COLOR_RESET}"
            echo -e "${COLOR_YELLOW}[WARNING] You may need to add it to your shell config (.bashrc / .zshrc):${COLOR_RESET}"
            echo -e "  export PATH=\"\$PATH:\$HOME/.local/bin\""
            ;;
    esac
fi

echo "Installing binary to: $INSTALL_DIR/lgtui"

# Fetch latest release info
REPO="ucmz851/lgtui"
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

# Install LGTUI binary
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
    if [ -f "icon.svg" ]; then
        cp "icon.svg" "$TMP_DIR/icon.svg"
    fi
else
    echo "Downloading LGTUI release ($LATEST_TAG) from GitHub..."
    download_success=0
    if command -v curl >/dev/null 2>&1; then
        if curl -sSL -f -o "$TMP_DIR/lgtui.tar.gz" "$DOWNLOAD_URL"; then
            download_success=1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if wget -q -O "$TMP_DIR/lgtui.tar.gz" "$DOWNLOAD_URL"; then
            download_success=1
        fi
    else
        echo -e "${COLOR_RED}[ERROR] Neither curl nor wget was found. Cannot download release.${COLOR_RESET}"
        exit 1
    fi

    if [ "$download_success" -ne 1 ]; then
        echo -e "${COLOR_RED}[ERROR] Failed to download release asset from: $DOWNLOAD_URL${COLOR_RESET}"
        echo -e "${COLOR_RED}[ERROR] This usually means release $LATEST_TAG has not been created yet or the GitHub Action release build is still in progress.${COLOR_RESET}"
        echo -e "${COLOR_YELLOW}[TIP] Try pushing your release tag now: git tag $LATEST_TAG && git push origin $LATEST_TAG${COLOR_RESET}"
        exit 1
    fi

    tar -xzf "$TMP_DIR/lgtui.tar.gz" -C "$TMP_DIR"
fi

# Copy binary to destination
cp "$TMP_DIR/lgtui" "$INSTALL_DIR/lgtui"
chmod +x "$INSTALL_DIR/lgtui"

# Copy icon to destination
if [ -f "$TMP_DIR/icon.svg" ] && [ -n "$ICON_DIR" ]; then
    echo "Installing application icon..."
    cp "$TMP_DIR/icon.svg" "$ICON_DIR/lgtui.svg"
    if [ "$(id -u)" -ne 0 ]; then
        cp "$TMP_DIR/icon.svg" "$HOME/.local/share/icons/lgtui.svg"
    else
        cp "$TMP_DIR/icon.svg" "/usr/share/icons/lgtui.svg"
    fi
fi

# Setup Desktop Entry file
echo "Installing desktop application entry..."
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR"

cat <<EOF > "$DESKTOP_DIR/lgtui.desktop"
[Desktop Entry]
Name=LGTUI
Comment=Linux Gaming Terminal UI
Exec=$INSTALL_DIR/lgtui
Icon=lgtui
Terminal=true
Type=Application
Categories=Game;Utility;
Keywords=wine;gaming;tui;proton;
EOF

chmod +x "$DESKTOP_DIR/lgtui.desktop"

echo -e "\n${COLOR_GREEN}=== LGTUI Binary Installed Successfully! ===${COLOR_RESET}"
echo -e "The LGTUI binary and application icon are now registered on your system."
echo ""
echo -e "${COLOR_YELLOW}${COLOR_BOLD}👉 NEXT STEP:${COLOR_RESET}"
echo -e "To configure Wine compatibility branches, Winetricks, MangoHud, and GameMode, run:"
echo -e "  ${COLOR_BLUE}${COLOR_BOLD}lgtui --install${COLOR_RESET}"
echo ""
echo -e "Once configured, simply launch the dashboard by typing ${COLOR_BLUE}${COLOR_BOLD}lgtui${COLOR_RESET} in your terminal."
