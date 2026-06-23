#!/bin/sh
# install_deps.sh - Linux Gaming Terminal UI (LGTUI) Dependency Installer
# Automatically detects the Linux distribution and installs the necessary
# components for a high-performance Linux gaming ecosystem:
# Wine, Winetricks, MangoHud, GameMode, and 32-bit Vulkan/graphics drivers.

# Unbuffer output for real-time TUI streaming
if command -v stdbuf >/dev/null 2>&1; then
    exec stdbuf -oL -eL "$0" "$@"
fi

echo "=== LGTUI Dependency Installer ==="
echo "[1/4] Detecting Linux Distribution..."

# Helper to verify command availability
has_cmd() {
    command -v "$1" >/dev/null 2>&1
}

# Determine operating system identifier
OS_ID=""
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_ID="$ID"
fi

# Fallback detection using package manager binaries
PM=""
if has_cmd pacman; then
    PM="pacman"
elif has_cmd dnf; then
    PM="dnf"
elif has_cmd apt-get; then
    PM="apt"
elif has_cmd zypper; then
    PM="zypper"
fi

if [ -z "$PM" ]; then
    echo "[ERROR] Unsupported distribution package manager."
    echo "[ERROR] Please manually install the following packages:"
    echo "  - Wine (staging preferred, with 32-bit support)"
    echo "  - Winetricks"
    echo "  - MangoHud"
    echo "  - GameMode"
    exit 1
fi

echo "Detected OS: $NAME ($OS_ID), Package Manager: $PM"
echo "[2/4] Verifying Root / Sudo Access..."

# Check root status or passwordless sudo privileges
SUDO=""
if [ "$(id -u)" -eq 0 ]; then
    echo "Running directly as root."
else
    if has_cmd sudo; then
        if sudo -n true 2>/dev/null; then
            echo "Non-interactive sudo authorization confirmed."
            SUDO="sudo"
        else
            echo "[ERROR] Root privileges are required to install dependencies."
            echo "[ERROR] Please run this installer with root access."
            echo "[ERROR] e.g., run LGTUI with sudo, or execute the script manually:"
            echo "      sudo ~/.config/lgtui/install_deps.sh"
            exit 1
        fi
    else
        echo "[ERROR] Root privileges are required, but 'sudo' was not found."
        echo "[ERROR] Please run this script as root."
        exit 1
    fi
fi

echo "[3/4] Installing dependencies..."

case "$PM" in
    pacman)
        echo "Synchronizing package databases..."
        $SUDO pacman -Sy --noconfirm

        # Check if the [multilib] repository is enabled for 32-bit libs
        if ! grep -q "^\[multilib\]" /etc/pacman.conf; then
            echo "[WARNING] multilib repository is not enabled in /etc/pacman.conf!"
            echo "[WARNING] 32-bit graphics drivers may fail to install."
        fi

        echo "Installing Arch packages: wine-staging, winetricks, mangohud, gamemode, 32-bit Vulkan/Mesa drivers..."
        # wine-staging: Bleeding-edge Wine implementation
        # winetricks: Windows library installer utility
        # mangohud: Vulkan and OpenGL overlay for monitoring FPS and systems
        # gamemode: Optimizes system performance on demand
        # vulkan-radeon, lib32-vulkan-radeon, lib32-mesa: Vulkan drivers for AMD/Mesa
        $SUDO pacman -S --needed --noconfirm \
            wine-staging \
            winetricks \
            mangohud \
            gamemode \
            vulkan-radeon \
            lib32-vulkan-radeon \
            lib32-mesa
        ;;

    dnf)
        echo "Installing Fedora packages: wine, winetricks, mangohud, gamemode, 32-bit Mesa/Vulkan drivers..."
        # wine, winetricks, mangohud, gamemode: Core gaming runtime and utilities
        # mesa-vulkan-drivers.i686, mesa-dri-drivers.i686, vulkan-loader.i686: 32-bit display drivers
        $SUDO dnf install -y \
            wine \
            winetricks \
            mangohud \
            gamemode \
            mesa-vulkan-drivers.i686 \
            mesa-dri-drivers.i686 \
            vulkan-loader.i686
        ;;

    apt)
        echo "Configuring multiarch for 32-bit packages..."
        $SUDO dpkg --add-architecture i386
        echo "Updating package repositories..."
        $SUDO apt-get update

        echo "Installing Debian/Ubuntu packages: wine64, wine32, winetricks, mangohud, gamemode..."
        # wine64 / wine32: 64-bit and 32-bit Wine interpreters
        # winetricks: Windows installer helper
        # mangohud / gamemode: Overlays and performance daemons
        $SUDO apt-get install -y \
            wine64 \
            wine32 \
            winetricks \
            mangohud \
            gamemode
        ;;

    zypper)
        echo "Installing openSUSE packages: wine, wine-32bit, winetricks, mangohud, gamemode..."
        # wine-32bit: Crucial for running 32-bit binaries under openSUSE
        $SUDO zypper --non-interactive install \
            wine \
            wine-32bit \
            winetricks \
            mangohud \
            gamemode
        ;;
esac

echo "[4/4] Verifying installations..."

MISSING=""
for cmd_name in wine winetricks; do
    if ! has_cmd "$cmd_name"; then
        MISSING="$MISSING $cmd_name"
    fi
done

if [ -n "$MISSING" ]; then
    echo "[ERROR] Verification failed. Missing executables:$MISSING"
    exit 1
fi

echo "=== System Dependencies Configured Successfully! ==="
exit 0
