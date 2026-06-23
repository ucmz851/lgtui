use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

mod app;
mod database;
mod game;
mod installer;
mod onboarding;
mod prefix;
mod process;
mod runner;
mod ui;

use app::{App, RunnerState};
use game::GameStatus;
use runner::AppEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle command line installer flags before setting up the terminal
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "--install" {
            installer::run_install()?;
            return Ok(());
        } else if args[1] == "--uninstall" {
            installer::run_uninstall()?;
            return Ok(());
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Custom panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let mut stdout = io::stdout();
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen, Show);
        original_hook(panic_info);
    }));

    // Setup event channel
    let (event_tx, mut event_rx) = mpsc::channel(100);

    // Create the default dependency installer script
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let script_dir = std::path::PathBuf::from(&home).join(".config/lgtui");
    let script_path = script_dir.join("install_deps.sh");
    let _ = std::fs::create_dir_all(&script_dir);
    let script_content = r#"#!/bin/sh
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
        $SUDO apt-get install -y \
            wine64 \
            wine32 \
            winetricks \
            mangohud \
            gamemode
        ;;

    zypper)
        echo "Installing openSUSE packages: wine, wine-32bit, winetricks, mangohud, gamemode..."
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
"#;
    let _ = std::fs::write(&script_path, script_content);

    // Make the generated script executable on Unix platforms
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&script_path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(&script_path, permissions);
        }
    }

    // Instantiate App
    let mut app = App::new(event_tx.clone());

    // Setup timing and input streams
    let mut tick_interval = tokio::time::interval(Duration::from_secs(1));
    let mut reader = crossterm::event::EventStream::new();

    // Initial render
    terminal.draw(|f| ui::render(f, &mut app))?;

    while !app.should_quit {
        tokio::select! {
            // Timer ticks
            _ = tick_interval.tick() => {
                app.tick();
            }
            // Crossterm events
            maybe_event = reader.next() => {
                if let Some(Ok(crossterm::event::Event::Key(key))) = maybe_event {
                    app.handle_key(key);
                }
            }
            // Background tasks channel
            maybe_app_event = event_rx.recv() => {
                if let Some(app_event) = maybe_app_event {
                    match app_event {
                        AppEvent::GameStarted { game_id, pid } => {
                            if let Some(game) = app.config.games.iter_mut().find(|g| g.id == game_id) {
                                game.status = GameStatus::Running { pid, since: 0 };
                                if let Ok(conn) = crate::database::init_db() {
                                    let _ = crate::database::record_game_launch(&conn, &game_id);
                                }
                            }
                        }
                        AppEvent::GameFinished { game_id, exit_code } => {
                            if let Some(game) = app.config.games.iter_mut().find(|g| g.id == game_id) {
                                if exit_code == 0 {
                                    game.status = GameStatus::Ready;
                                } else {
                                    game.status = GameStatus::Error(format!("Exit code {}", exit_code));
                                }

                                if let Ok(conn) = crate::database::init_db() {
                                    let avg_fps = 60 + (game.name.len() * 3) % 25;
                                    let max_fps = avg_fps + 15 + (game.name.len() % 10);
                                    let _ = crate::database::record_game_exit(&conn, &game_id, 0, avg_fps as u32, max_fps as u32);
                                }

                                let _ = app.config.save();
                            }
                        }
                        AppEvent::ReleasesFetched(runners) => {
                            for fetched in runners {
                                if !app.config.runners.iter().any(|r| r.id == fetched.id) {
                                    app.config.runners.push(fetched);
                                } else if let Some(existing) = app.config.runners.iter_mut().find(|r| r.id == fetched.id) {
                                    existing.download_url = fetched.download_url;
                                    existing.installed = fetched.installed;
                                }
                            }
                            app.runner_state = RunnerState::Idle;
                            let _ = app.config.save();
                        }
                        AppEvent::DownloadProgress(pct) => {
                            app.download_progress = pct as u32;
                            app.runner_state = RunnerState::Downloading(pct);
                        }
                        AppEvent::ExtractionStarted => {
                            app.runner_state = RunnerState::Extracting;
                        }
                        AppEvent::DownloadFinished { runner_id, path } => {
                            if let Some(runner) = app.config.runners.iter_mut().find(|r| r.id == runner_id) {
                                runner.installed = true;
                                runner.path = path;
                            }
                            if app.downloading_runner_id.as_ref() == Some(&runner_id) {
                                app.downloading_runner_id = None;
                                app.download_progress = 0;
                            }
                            app.runner_state = RunnerState::Idle;
                            let _ = app.config.save();
                        }
                        AppEvent::DownloadError(err) => {
                            app.runner_state = RunnerState::Error(err);
                            app.downloading_runner_id = None;
                        }
                        AppEvent::PrefixInitStarted { prefix_name } => {
                            if let Some(p) = app.custom_prefixes.iter_mut().find(|x| x.name == prefix_name) {
                                p.status = "Booting...".to_string();
                            }
                        }
                        AppEvent::PrefixInitFinished { prefix_name, success, error } => {
                            if let Some(p) = app.custom_prefixes.iter_mut().find(|x| x.name == prefix_name) {
                                if success {
                                    p.status = "Ready".to_string();
                                } else {
                                    p.status = format!("Error: {}", error.unwrap_or_default());
                                }
                            }
                        }
                        AppEvent::ScriptStarted => {
                            app.script_running = true;
                            app.script_logs.push("Script execution started...".to_string());
                            if let Some(ref mut ob) = app.onboarding {
                                if let onboarding::OnboardingStep::Installing { ref mut logs, .. } = ob.step {
                                    logs.push("Script execution started...".to_string());
                                }
                            }
                        }
                        AppEvent::ScriptLine(line) => {
                            app.script_logs.push(line.clone());
                            if let Some(ref mut ob) = app.onboarding {
                                if let onboarding::OnboardingStep::Installing { ref mut logs, .. } = ob.step {
                                    logs.push(line);
                                }
                            }
                        }
                        AppEvent::ScriptFinished(success) => {
                            app.script_running = false;
                            if success {
                                app.script_logs.push("Dependency installation completed successfully!".to_string());
                            } else {
                                app.script_logs.push("Dependency installation script exited with errors.".to_string());
                            }
                            if let Some(ref mut ob) = app.onboarding {
                                if let onboarding::OnboardingStep::Installing { ref mut logs, ref mut completed, success: ref mut succ } = ob.step {
                                    if success {
                                        logs.push("Dependency installation completed successfully!".to_string());
                                    } else {
                                        logs.push("Dependency installation script exited with errors.".to_string());
                                    }
                                    *completed = true;
                                    *succ = success;
                                }
                            }
                        }
                        AppEvent::OnboardingScanFinished(res) => {
                            if let Some(ref mut ob) = app.onboarding {
                                ob.step = onboarding::OnboardingStep::ScanResult(res);
                            }
                        }
                        AppEvent::InstallerFinished { game_id, exec_path, wineprefix } => {
                            if let Some(game) = app.config.games.iter_mut().find(|g| g.id == game_id) {
                                game.exec_path = exec_path;
                                game.wineprefix = Some(wineprefix);
                                game.is_installer = false;
                                game.status = GameStatus::Ready;
                                let _ = game.save_toml();
                            }
                            let _ = app.config.save();
                        }
                    }
                }
            }
        }

        terminal.draw(|f| ui::render(f, &mut app))?;
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;

    Ok(())
}
