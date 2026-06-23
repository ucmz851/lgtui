#!/bin/bash
# uninstall.sh - Cleaner for LGTUI

set -e

# ANSI Color Codes
COLOR_GREEN='\033[0;32m'
COLOR_BLUE='\033[0;34m'
COLOR_YELLOW='\033[1;33m'
COLOR_RED='\033[0;31m'
COLOR_RESET='\033[0m'

# Run native Rust uninstallation configuration purge
INSTALL_DIR=""
if [ -f "/usr/local/bin/lgtui" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -f "$HOME/.local/bin/lgtui" ]; then
    INSTALL_DIR="$HOME/.local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
fi

if [ -x "$INSTALL_DIR/lgtui" ]; then
    "$INSTALL_DIR/lgtui" --uninstall
fi

# Remove binary file
if [ -f "$INSTALL_DIR/lgtui" ]; then
    echo "Removing binary: $INSTALL_DIR/lgtui"
    if [ "$(id -u)" -eq 0 ]; then
        sudo rm -f "$INSTALL_DIR/lgtui"
    else
        rm -f "$INSTALL_DIR/lgtui"
    fi
fi

echo -e "${COLOR_GREEN}=== LGTUI Uninstalled Successfully! ===${COLOR_RESET}"
