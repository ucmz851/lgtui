use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::sync::mpsc::Sender;

use crate::game::{Game, Runner};
use crate::runner::AppEvent;

/// Launches a game or an installer in the background.
pub fn launch_game_or_installer(
    mut game: Game,
    runner: Option<Runner>,
    event_tx: Sender<AppEvent>,
) {
    tokio::spawn(async move {
        let runner_path = runner
            .as_ref()
            .map(|r| r.path.clone())
            .unwrap_or_else(|| "/usr/bin/wine".to_string());

        if !game.is_installer {
            // Normal game launch path
            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("scripts/launch_game.sh");

            // Pass parameters via environment variables
            cmd.env("RUNNER", &runner_path)
                .env("EXEC_PATH", &game.exec_path)
                .env("DXVK", if game.dxvk { "1" } else { "0" })
                .env("VKD3D", if game.vkd3d { "1" } else { "0" })
                .env("MANGOHUD", if game.mangohud { "1" } else { "0" })
                .env("GAMEMODE", if game.gamemode { "1" } else { "0" });

            if let Some(prefix) = &game.wineprefix {
                cmd.env("WINEPREFIX", prefix);
            }

            let args_str = game.args.join(" ");
            cmd.env("GAME_ARGS", args_str);

            cmd.stdout(Stdio::null()).stderr(Stdio::null());

            match cmd.spawn() {
                Ok(mut child) => {
                    let pid = child.id().unwrap_or(0);
                    let _ = event_tx
                        .send(AppEvent::GameStarted {
                            game_id: game.id.clone(),
                            pid,
                        })
                        .await;

                    match child.wait().await {
                        Ok(status) => {
                            let exit_code = status.code().unwrap_or(0);
                            let _ = event_tx
                                .send(AppEvent::GameFinished {
                                    game_id: game.id.clone(),
                                    exit_code,
                                })
                                .await;
                        }
                        Err(_) => {
                            let _ = event_tx
                                .send(AppEvent::GameFinished {
                                    game_id: game.id.clone(),
                                    exit_code: -1,
                                })
                                .await;
                        }
                    }
                }
                Err(_) => {
                    let _ = event_tx
                        .send(AppEvent::GameFinished {
                            game_id: game.id.clone(),
                            exit_code: -1,
                        })
                        .await;
                }
            }
        } else {
            // Installer pipeline path
            let _ = event_tx
                .send(AppEvent::GameStarted {
                    game_id: game.id.clone(),
                    pid: std::process::id(), // Temporary pid
                })
                .await;

            // 1. Determine temporary prefix path
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let base_dir = PathBuf::from(&home).join(".local/share/lgtui");
            let temp_prefix_path = base_dir
                .join("prefixes")
                .join(format!("temp_install_{}", game.id));

            // Ensure temp prefix directory is clean/created
            if temp_prefix_path.exists() {
                let _ = tokio::fs::remove_dir_all(&temp_prefix_path).await;
            }
            if let Err(_e) = tokio::fs::create_dir_all(&temp_prefix_path).await {
                let _ = event_tx
                    .send(AppEvent::GameFinished {
                        game_id: game.id.clone(),
                        exit_code: -1,
                    })
                    .await;
                return;
            }

            // 2. Perform silent prefix boot
            let runner_str = runner_path.clone();
            let mut boot_cmd = if runner_str.contains("proton") {
                let mut c = tokio::process::Command::new(&runner_path);
                c.args(["run", "wine", "boot", "-u"]);
                c
            } else {
                let mut c = tokio::process::Command::new(&runner_path);
                c.args(["boot", "-u"]);
                c
            };

            boot_cmd
                .env("WINEPREFIX", &temp_prefix_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            let boot_success = match boot_cmd.spawn() {
                Ok(mut child) => child.wait().await.map(|s| s.success()).unwrap_or(false),
                Err(_) => false,
            };

            if !boot_success {
                let _ = event_tx
                    .send(AppEvent::GameFinished {
                        game_id: game.id.clone(),
                        exit_code: -1,
                    })
                    .await;
                return;
            }

            // 3. Run the installer executable
            let mut install_cmd = if runner_str.contains("proton") {
                let mut c = tokio::process::Command::new(&runner_path);
                c.args(["run", "wine", &game.exec_path]);
                c
            } else {
                let mut c = tokio::process::Command::new(&runner_path);
                c.arg(&game.exec_path);
                c
            };

            install_cmd
                .env("WINEPREFIX", &temp_prefix_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            let install_status = match install_cmd.spawn() {
                Ok(mut child) => child.wait().await,
                Err(e) => Err(e),
            };

            let install_success = match install_status {
                Ok(status) => status.success(),
                Err(_) => false,
            };

            if !install_success {
                let _ = event_tx
                    .send(AppEvent::GameFinished {
                        game_id: game.id.clone(),
                        exit_code: -1,
                    })
                    .await;
                return;
            }

            // 4. Recursively scan drive_c
            let drive_c = temp_prefix_path.join("drive_c");
            let mut exe_files = Vec::new();
            scan_exe_files(&drive_c, &mut exe_files);

            let mut filtered: Vec<_> = exe_files
                .iter()
                .filter(|(path, _)| {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        !is_filtered_exe(name)
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            filtered.sort_by_key(|b| std::cmp::Reverse(b.1));

            let chosen_exec = if !filtered.is_empty() {
                Some(filtered[0].0.clone())
            } else if !exe_files.is_empty() {
                let mut sorted_all = exe_files.clone();
                sorted_all.sort_by_key(|b| std::cmp::Reverse(b.1));
                Some(sorted_all[0].0.clone())
            } else {
                None
            };

            let chosen_exec_path = match chosen_exec {
                Some(p) => p,
                None => {
                    let _ = event_tx
                        .send(AppEvent::GameFinished {
                            game_id: game.id.clone(),
                            exit_code: -2, // Code -2: Executable not found after install
                        })
                        .await;
                    return;
                }
            };

            // 5. Determine final prefix destination
            let final_prefix_path = if let Some(custom_prefix_str) = &game.wineprefix {
                PathBuf::from(custom_prefix_str)
            } else {
                base_dir.join("prefixes").join(&game.id)
            };

            // If final directory already exists, remove it first
            if final_prefix_path.exists() {
                let _ = tokio::fs::remove_dir_all(&final_prefix_path).await;
            }

            // Move prefix from temp to final destination
            if move_dir(temp_prefix_path.clone(), final_prefix_path.clone())
                .await
                .is_err()
            {
                let _ = event_tx
                    .send(AppEvent::GameFinished {
                        game_id: game.id.clone(),
                        exit_code: -3, // Code -3: Prefix relocation failed
                    })
                    .await;
                return;
            }

            // 6. Map executable path from temp prefix to final prefix
            let rel_path = match chosen_exec_path.strip_prefix(&temp_prefix_path) {
                Ok(rp) => rp,
                Err(_) => {
                    let _ = event_tx
                        .send(AppEvent::GameFinished {
                            game_id: game.id.clone(),
                            exit_code: -4,
                        })
                        .await;
                    return;
                }
            };

            let final_exec_path = final_prefix_path.join(rel_path);

            // Update game configuration
            game.exec_path = final_exec_path.to_string_lossy().to_string();
            game.wineprefix = Some(final_prefix_path.to_string_lossy().to_string());
            game.is_installer = false; // Pipeline complete, now it's a regular game!

            let _ = game.save_toml();

            let _ = event_tx
                .send(AppEvent::InstallerFinished {
                    game_id: game.id.clone(),
                    exec_path: game.exec_path.clone(),
                    wineprefix: game.wineprefix.clone().unwrap(),
                })
                .await;
        }
    });
}

fn scan_exe_files(dir: &Path, files: &mut Vec<(PathBuf, u64)>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_exe_files(&path, files);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("exe") {
                        if let Ok(metadata) = entry.metadata() {
                            files.push((path, metadata.len()));
                        }
                    }
                }
            }
        }
    }
}

fn is_filtered_exe(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.starts_with("unins")
        || lower.contains("uninstall")
        || lower.contains("redist")
        || lower.contains("vc_redist")
        || lower.contains("vcredist")
        || lower.contains("dxsetup")
        || lower.contains("helper")
        || lower.contains("crashhandler")
        || lower.contains("crashpad")
        || lower.contains("register")
        || lower.contains("activation")
        || lower.contains("setup")
        || lower.contains("install")
}

async fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    tokio::fs::create_dir_all(&dst).await?;
    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let ty = entry.file_type().await?;
        if ty.is_dir() {
            Box::pin(copy_dir_all(entry.path(), dst.join(entry.file_name()))).await?;
        } else {
            tokio::fs::copy(entry.path(), dst.join(entry.file_name())).await?;
        }
    }
    Ok(())
}

async fn move_dir(src: PathBuf, dst: PathBuf) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if tokio::fs::rename(&src, &dst).await.is_err() {
        copy_dir_all(&src, &dst).await?;
        let _ = tokio::fs::remove_dir_all(&src).await;
    }
    Ok(())
}
