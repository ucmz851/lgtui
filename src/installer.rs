use std::io::{self, Write};
use std::process::Command;

fn print_banner() {
    println!("\x1b[1;35m");
    println!("  _      _____ _______ _    _ _____ ");
    println!(" | |    / ____|__   __| |  | |_   _|");
    println!(" | |   | |  __   | |  | |  | | | |  ");
    println!(" | |   | | |_ |  | |  | |  | | | |  ");
    println!(" | |___| |__| |  | |  | |__| |_| |_ ");
    println!(" |______\\_____|  |_|   \\____/|_____|");
    println!("\x1b[0m");
    println!("\x1b[1m=== LGTUI Native Rust Installation Wizard ===\x1b[0m");
    println!();
}

fn ask_yn(prompt: &str, default: bool) -> bool {
    let choices = if default { "[Y/n]" } else { "[y/N]" };
    print!("\x1b[1;36m{} {} \x1b[0m", prompt, choices);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim().to_lowercase();

    if trimmed.is_empty() {
        return default;
    }

    trimmed == "y" || trimmed == "yes"
}

fn ask_choice(prompt: &str, default: &str) -> String {
    print!("\x1b[1;36m{} (Default: {}): \x1b[0m", prompt, default);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();

    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn detect_pm() -> &'static str {
    if std::path::Path::new("/usr/bin/pacman").exists()
        || std::path::Path::new("/bin/pacman").exists()
    {
        "pacman"
    } else if std::path::Path::new("/usr/bin/apt-get").exists()
        || std::path::Path::new("/bin/apt-get").exists()
    {
        "apt"
    } else if std::path::Path::new("/usr/bin/dnf").exists()
        || std::path::Path::new("/bin/dnf").exists()
    {
        "dnf"
    } else {
        "unknown"
    }
}

pub fn run_install() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    let pm = detect_pm();
    if pm == "unknown" {
        println!(
            "\x1b[1;33m[WARNING] Supported package manager (pacman, apt, dnf) not detected.\x1b[0m"
        );
        println!("\x1b[1;33m[WARNING] Skipping dependency installation. Please install Wine, Winetricks, MangoHud, and GameMode manually.\x1b[0m");
        return Ok(());
    }

    println!("\x1b[1;32m------------------------------------------------------------\x1b[0m");
    println!("\x1b[1m🎮 Linux Gaming Compatibility Setup Wizard\x1b[0m");
    println!("Let's configure compatibility runners, tools, and HUD overlays.");
    println!("\x1b[1;32m------------------------------------------------------------\x1b[0m");

    // Questions
    let wine_install = ask_yn("Install Wine Compatibility Layer?", true);
    let mut wine_choice = "2".to_string();
    if wine_install {
        wine_choice = ask_choice(
            "Choose Wine branch: [1] Stable  [2] Staging (Recommended for gaming)",
            "2",
        );
    }

    let tricks_install = ask_yn(
        "Install Winetricks (helper to download DLLs, fonts, and runtime libraries)?",
        true,
    );
    let hud_install = ask_yn(
        "Install MangoHud (high-performance overlay for FPS, CPU/GPU, and VRAM monitoring)?",
        true,
    );
    let mode_install = ask_yn(
        "Install Feral GameMode (optimizes Linux system priorities dynamically on game launch)?",
        true,
    );

    let mut multiarch_install = false;
    if pm == "apt" {
        multiarch_install = ask_yn(
            "Enable 32-bit architecture & libraries (mandatory for older/Steam games)?",
            true,
        );
    }

    println!("\n\x1b[1;34m--- Applying Package Choices ---\x1b[0m");

    if multiarch_install {
        println!("\x1b[1;33mEnabling 32-bit architecture...\x1b[0m");
        let status = Command::new("sudo")
            .arg("dpkg")
            .arg("--add-architecture")
            .arg("i386")
            .status()?;
        if status.success() {
            let _ = Command::new("sudo").arg("apt-get").arg("update").status();
        }
    }

    let mut packages = Vec::new();

    if wine_install {
        match pm {
            "pacman" => {
                if wine_choice == "2" {
                    packages.push("wine-staging");
                } else {
                    packages.push("wine");
                }
                packages.push("wine-mono");
                packages.push("wine-gecko");
            }
            "apt" => {
                if wine_choice == "2" {
                    packages.push("wine-development");
                } else {
                    packages.push("wine");
                }
            }
            "dnf" => {
                packages.push("wine");
                packages.push("wine-mono-core");
                packages.push("wine-gecko");
            }
            _ => {}
        }
    }

    if tricks_install {
        packages.push("winetricks");
    }

    if hud_install {
        packages.push("mangohud");
        if pm == "pacman" {
            packages.push("lib32-mangohud");
        }
    }

    if mode_install {
        packages.push("gamemode");
        if pm == "pacman" {
            packages.push("lib32-gamemode");
        }
    }

    if !packages.is_empty() {
        println!(
            "\x1b[1;36mInstalling selected packages: \x1b[1m{:?}\x1b[0m",
            packages
        );
        let status = match pm {
            "pacman" => Command::new("sudo")
                .arg("pacman")
                .arg("-S")
                .arg("--noconfirm")
                .arg("--needed")
                .args(&packages)
                .status()?,
            "apt" => Command::new("sudo")
                .arg("apt-get")
                .arg("install")
                .arg("-y")
                .args(&packages)
                .status()?,
            "dnf" => Command::new("sudo")
                .arg("dnf")
                .arg("install")
                .arg("-y")
                .args(&packages)
                .status()?,
            _ => return Ok(()),
        };

        if status.success() {
            println!("\x1b[1;32m[SUCCESS] Dependencies installed successfully.\x1b[0m");
        } else {
            println!("\x1b[1;31m[ERROR] Failed to install packages.\x1b[0m");
        }
    } else {
        println!("\x1b[1;33mNo packages selected for installation.\x1b[0m");
    }

    Ok(())
}

pub fn run_uninstall() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;31m\x1b[1m=== LGTUI Native Rust Uninstaller ===\x1b[0m");
    println!();

    if !ask_yn("Are you sure you want to uninstall LGTUI?", false) {
        println!("Uninstallation canceled.");
        return Ok(());
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    // 1. Remove desktop file
    let desktop_path =
        std::path::PathBuf::from(&home).join(".local/share/applications/lgtui.desktop");
    if desktop_path.exists() {
        println!("Removing desktop entry: {:?}", desktop_path);
        let _ = std::fs::remove_file(desktop_path);
    }

    // 2. Remove icons
    let user_icon =
        std::path::PathBuf::from(&home).join(".local/share/icons/hicolor/scalable/apps/lgtui.svg");
    if user_icon.exists() {
        println!("Removing icon: {:?}", user_icon);
        let _ = std::fs::remove_file(user_icon);
    }

    let user_fallback_icon = std::path::PathBuf::from(&home).join(".local/share/icons/lgtui.svg");
    if user_fallback_icon.exists() {
        let _ = std::fs::remove_file(user_fallback_icon);
    }

    // Global icons (need sudo)
    let global_icon = std::path::Path::new("/usr/share/icons/hicolor/scalable/apps/lgtui.svg");
    if global_icon.exists() {
        println!("Removing global icon: {:?}", global_icon);
        let _ = Command::new("sudo")
            .arg("rm")
            .arg("-f")
            .arg(global_icon)
            .status();
    }

    let global_fallback_icon = std::path::Path::new("/usr/share/icons/lgtui.svg");
    if global_fallback_icon.exists() {
        let _ = Command::new("sudo")
            .arg("rm")
            .arg("-f")
            .arg(global_fallback_icon)
            .status();
    }

    // 3. Option to purge database and configs
    if ask_yn(
        "Do you want to delete your SQLite database containing game settings and playtime stats?",
        false,
    ) {
        let db_dir = std::path::PathBuf::from(&home).join(".local/share/lgtui");
        let config_dir = std::path::PathBuf::from(&home).join(".config/lgui");

        if db_dir.exists() {
            println!("Purging directory: {:?}", db_dir);
            let _ = std::fs::remove_dir_all(db_dir);
        }
        if config_dir.exists() {
            println!("Purging directory: {:?}", config_dir);
            let _ = std::fs::remove_dir_all(config_dir);
        }
        println!(
            "\x1b[1;32m[CLEANUP] SQLite database and configurations purged successfully.\x1b[0m"
        );
    }

    println!("\n\x1b[1;32m\x1b[1m=== LGTUI Configurations Cleaned Successfully! ===\x1b[0m");
    Ok(())
}
