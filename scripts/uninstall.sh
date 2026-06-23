#!/bin/bash
# uninstall.sh - Cleaner for LGTUI (Linux Gaming Terminal UI)

set -e

# ANSI Color Codes
COLOR_GREEN='\033[0;32m'
COLOR_CYAN='\033[0;36m'
COLOR_BLUE='\033[0;34m'
COLOR_YELLOW='\033[1;33m'
COLOR_RED='\033[0;31m'
COLOR_BOLD='\033[1m'
COLOR_RESET='\033[0m'

echo -e "${COLOR_RED}${COLOR_BOLD}=== LGTUI Uninstaller ===${COLOR_RESET}"
echo ""

# Helper function for prompts
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

# Determine installation locations
INSTALL_DIR=""
ICON_DIR=""
GLOBAL_INSTALL=0

# Detect if the binary is installed globally or locally
if [ -f "/usr/local/bin/lgtui" ]; then
    INSTALL_DIR="/usr/local/bin"
    ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
    GLOBAL_INSTALL=1
elif [ -f "$HOME/.local/bin/lgtui" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
else
    # Fallback/Default check
    if [ "$(id -u)" -eq 0 ]; then
        INSTALL_DIR="/usr/local/bin"
        ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
        GLOBAL_INSTALL=1
    else
        INSTALL_DIR="$HOME/.local/bin"
        ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
    fi
fi

# 1. Remove binary file
if [ -f "$INSTALL_DIR/lgtui" ]; then
    echo "Removing binary: $INSTALL_DIR/lgtui"
    if [ "$GLOBAL_INSTALL" -eq 1 ]; then
        sudo rm -f "$INSTALL_DIR/lgtui"
    else
        rm -f "$INSTALL_DIR/lgtui"
    fi
else
    echo "Binary lgtui not found in $INSTALL_DIR"
fi

# 2. Remove desktop icon
if [ -n "$ICON_DIR" ]; then
    if [ -f "$ICON_DIR/lgtui.svg" ]; then
        echo "Removing icon: $ICON_DIR/lgtui.svg"
        if [ "$GLOBAL_INSTALL" -eq 1 ]; then
            sudo rm -f "$ICON_DIR/lgtui.svg"
        else
            rm -f "$ICON_DIR/lgtui.svg"
        fi
    fi
    # Remove fallback icon
    if [ -f "$HOME/.local/share/icons/lgtui.svg" ]; then
        rm -f "$HOME/.local/share/icons/lgtui.svg"
    fi
    if [ -f "/usr/share/icons/lgtui.svg" ]; then
        sudo rm -f "/usr/share/icons/lgtui.svg"
    fi
fi

# 3. Remove desktop application entry
DESKTOP_FILE="$HOME/.local/share/applications/lgtui.desktop"
if [ -f "$DESKTOP_FILE" ]; then
    echo "Removing desktop entry: $DESKTOP_FILE"
    rm -f "$DESKTOP_FILE"
fi

# 4. Optional: Purge database and configs
echo ""
PURGE_DATA=$(ask_yn "Do you want to delete your SQLite database containing game settings and playtime stats?" "n")
if [ "$PURGE_DATA" = "y" ]; then
    DB_DIR="$HOME/.local/share/lgtui"
    CONFIG_DIR="$HOME/.config/lgui"
    
    if [ -d "$DB_DIR" ]; then
        echo "Purging directory: $DB_DIR"
        rm -rf "$DB_DIR"
    fi
    
    if [ -d "$CONFIG_DIR" ]; then
        echo "Purging directory: $CONFIG_DIR"
        rm -rf "$CONFIG_DIR"
    fi
    echo -e "${COLOR_GREEN}[CLEANUP] Settings and statistics database purged successfully.${COLOR_RESET}"
else
    echo -e "${COLOR_YELLOW}[INFO] Your SQLite game library database and settings were preserved.${COLOR_RESET}"
fi

echo -e "\n${COLOR_GREEN}${COLOR_BOLD}=== LGTUI Uninstalled Successfully! ===${COLOR_RESET}"
