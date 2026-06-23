#!/bin/bash
# install.sh - Pre-compiled binary installer and interactive environment setup for LGTUI

set -e

# ANSI Color Codes
COLOR_GREEN='\033[0;32m'
COLOR_CYAN='\033[0;36m'
COLOR_BLUE='\033[0;34m'
COLOR_YELLOW='\033[1;33m'
COLOR_RED='\033[0;31m'
COLOR_MAGENTA='\033[0;35m'
COLOR_BOLD='\033[1m'
COLOR_RESET='\033[0m'

# Welcome Screen with ASCII Art
clear
echo -e "${COLOR_MAGENTA}${COLOR_BOLD}"
echo "  _      _____ _______ _    _ _____ "
echo " | |    / ____|__   __| |  | |_   _|"
echo " | |   | |  __   | |  | |  | | | |  "
echo " | |   | | |_ |  | |  | |  | | | |  "
echo " | |___| |__| |  | |  | |__| |_| |_ "
echo " |______\\_____|  |_|   \\____/|_____|"
echo -e "${COLOR_RESET}"
echo -e "${COLOR_BOLD}=== LGTUI Interactive Installation & System Setup ===${COLOR_RESET}"
echo ""

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

# Helper functions for prompts
ask_yn() {
    local prompt="$1"
    local default="$2" # y or n
    local choice
    
    if [ "$default" = "y" ]; then
        prompt="$prompt [Y/n]: "
    else
        prompt="$prompt [y/N]: "
    fi
    
    printf "${COLOR_CYAN}${COLOR_BOLD}%s${COLOR_RESET}" "$prompt"
    read -r choice
    
    if [ -z "$choice" ]; then
        choice="$default"
    fi
    
    case "$choice" in
        [yY]|[yY][eE][sS]) echo "y" ;;
        *) echo "n" ;;
    esac
}

ask_choice() {
    local prompt="$1"
    local default="$2"
    local choice
    
    printf "${COLOR_CYAN}${COLOR_BOLD}%s (Default: %s): ${COLOR_RESET}" "$prompt" "$default"
    read -r choice
    
    if [ -z "$choice" ]; then
        choice="$default"
    fi
    echo "$choice"
}

detect_package_manager() {
    if command -v pacman >/dev/null 2>&1; then
        echo "pacman"
    elif command -v apt-get >/dev/null 2>&1; then
        echo "apt"
    elif command -v dnf >/dev/null 2>&1; then
        echo "dnf"
    else
        echo "unknown"
    fi
}

# 1. Start Interactive Dependency Wizard
PM=$(detect_package_manager)

if [ "$PM" = "unknown" ]; then
    echo -e "${COLOR_YELLOW}[WARNING] Supported package manager (pacman, apt, dnf) not detected.${COLOR_RESET}"
    echo -e "${COLOR_YELLOW}[WARNING] Skipping interactive dependency installer wizard.${COLOR_RESET}"
else
    echo -e "${COLOR_GREEN}------------------------------------------------------------${COLOR_RESET}"
    echo -e "${COLOR_BOLD}🎮 Linux Gaming Compatibility Setup Wizard${COLOR_RESET}"
    echo -e "Let's configure compatibility runners, tools, and HUD overlays."
    echo -e "${COLOR_GREEN}------------------------------------------------------------${COLOR_RESET}"
    
    # Question: Install Wine
    WINE_INSTALL=$(ask_yn "Install Wine Compatibility Layer?" "y")
    WINE_CHOICE="2"
    if [ "$WINE_INSTALL" = "y" ]; then
        WINE_CHOICE=$(ask_choice "Choose Wine branch: [1] Stable  [2] Staging (Recommended for gaming)" "2")
    fi
    
    # Question: Install Winetricks
    TRICKS_INSTALL=$(ask_yn "Install Winetricks (helper to download DLLs, fonts, and runtime libraries)?" "y")
    
    # Question: Install MangoHud
    HUD_INSTALL=$(ask_yn "Install MangoHud (high-performance overlay for FPS, CPU/GPU, and VRAM monitoring)?" "y")
    
    # Question: Install GameMode
    MODE_INSTALL=$(ask_yn "Install Feral GameMode (optimizes Linux system priorities dynamically on game launch)?" "y")
    
    # Question: Enable Multi-arch
    MULTIARCH_INSTALL="n"
    if [ "$PM" = "apt" ]; then
        MULTIARCH_INSTALL=$(ask_yn "Enable 32-bit architecture & libraries (mandatory for older/Steam games)?" "y")
    fi

    # Execute installs
    echo -e "\n${COLOR_BLUE}${COLOR_BOLD}--- Applying Package Choices ---${COLOR_RESET}"
    
    PACKAGES=""
    
    if [ "$MULTIARCH_INSTALL" = "y" ]; then
        echo -e "${COLOR_YELLOW}Enabling 32-bit architecture...${COLOR_RESET}"
        sudo dpkg --add-architecture i386
        sudo apt-get update
    fi
    
    if [ "$WINE_INSTALL" = "y" ]; then
        if [ "$PM" = "pacman" ]; then
            if [ "$WINE_CHOICE" = "2" ]; then
                PACKAGES="$PACKAGES wine-staging"
            else
                PACKAGES="$PACKAGES wine"
            fi
            PACKAGES="$PACKAGES wine-mono wine-gecko"
        elif [ "$PM" = "apt" ]; then
            if [ "$WINE_CHOICE" = "2" ]; then
                PACKAGES="$PACKAGES wine-development"
            else
                PACKAGES="$PACKAGES wine"
            fi
        elif [ "$PM" = "dnf" ]; then
            PACKAGES="$PACKAGES wine wine-mono-core wine-gecko"
        fi
    fi
    
    if [ "$TRICKS_INSTALL" = "y" ]; then
        PACKAGES="$PACKAGES winetricks"
    fi
    
    if [ "$HUD_INSTALL" = "y" ]; then
        PACKAGES="$PACKAGES mangohud"
        if [ "$PM" = "pacman" ]; then
            PACKAGES="$PACKAGES lib32-mangohud"
        fi
    fi
    
    if [ "$MODE_INSTALL" = "y" ]; then
        if [ "$PM" = "pacman" ]; then
            PACKAGES="$PACKAGES gamemode lib32-gamemode"
        elif [ "$PM" = "apt" ]; then
            PACKAGES="$PACKAGES gamemode"
        elif [ "$PM" = "dnf" ]; then
            PACKAGES="$PACKAGES gamemode"
        fi
    fi

    if [ -n "$PACKAGES" ]; then
        echo -e "${COLOR_CYAN}Installing selected packages: ${COLOR_BOLD}$PACKAGES${COLOR_RESET}"
        if [ "$PM" = "pacman" ]; then
            sudo pacman -S --noconfirm --needed $PACKAGES
        elif [ "$PM" = "apt" ]; then
            sudo apt-get install -y $PACKAGES
        elif [ "$PM" = "dnf" ]; then
            sudo dnf install -y $PACKAGES
        fi
        echo -e "${COLOR_GREEN}[SUCCESS] Dependencies installed successfully.${COLOR_RESET}"
    else
        echo -e "${COLOR_YELLOW}No additional compatibility packages selected.${COLOR_RESET}"
    fi
    echo ""
fi

# 2. Determine install destination
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

echo -e "${COLOR_BLUE}${COLOR_BOLD}--- Installing LGTUI Binary ---${COLOR_RESET}"
echo "Destination directory: $INSTALL_DIR"

# 3. Fetch latest release info
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

# 4. Install LGTUI binary
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

# 5. Setup Desktop Entry file
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

echo -e "\n${COLOR_GREEN}${COLOR_BOLD}=== Installation Completed Successfully! ===${COLOR_RESET}"
echo -e "You can launch LGTUI by typing ${COLOR_BLUE}${COLOR_BOLD}lgtui${COLOR_RESET} in your terminal,"
echo -e "or run it directly from your desktop applications menu."
echo -e "Have fun gaming! 🚀"
