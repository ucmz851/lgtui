use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::sync::mpsc;

use crate::runner::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinePrefix {
    pub name: String,
    pub path: PathBuf,
    pub architecture: String, // "win32" or "win64"
    #[serde(skip)]
    pub status: String, // "Ready", "Booting...", or "Not Initialized"
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixConfig {
    pub prefixes: Vec<WinePrefix>,
}

#[allow(dead_code)]
impl PrefixConfig {
    pub fn get_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/lgui/prefixes.toml")
    }

    pub fn load_or_create() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<PrefixConfig>(&content) {
                    let mut config = config;
                    for p in &mut config.prefixes {
                        // Check if the prefix directory exists and contains user.reg to determine if it is initialized
                        let reg_file = p.path.join("user.reg");
                        p.status = if reg_file.exists() {
                            "Ready".to_string()
                        } else {
                            "Not Initialized".to_string()
                        };
                    }
                    return config;
                }
            }
        }
        let config = PrefixConfig {
            prefixes: Vec::new(),
        };
        let _ = config.save();
        config
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::other(format!("Failed to serialize prefix config to TOML: {}", e))
        })?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Initializes a Wine prefix asynchronously running `wine boot -u` in the background.
pub fn initialize_prefix(
    prefix: WinePrefix,
    runner_path: PathBuf,
    event_tx: mpsc::Sender<AppEvent>,
) {
    tokio::spawn(async move {
        let _ = event_tx
            .send(AppEvent::PrefixInitStarted {
                prefix_name: prefix.name.clone(),
            })
            .await;

        let runner_str = runner_path.to_string_lossy();
        let mut cmd = if runner_str.contains("proton") {
            let mut c = tokio::process::Command::new(&runner_path);
            c.args(["run", "wine", "boot", "-u"]);
            c
        } else {
            let mut c = tokio::process::Command::new(&runner_path);
            c.args(["boot", "-u"]);
            c
        };

        // Inject configuration environment variables
        cmd.env("WINEARCH", &prefix.architecture)
            .env("WINEPREFIX", &prefix.path)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => match child.wait().await {
                Ok(status) => {
                    if status.success() {
                        let _ = event_tx
                            .send(AppEvent::PrefixInitFinished {
                                prefix_name: prefix.name.clone(),
                                success: true,
                                error: None,
                            })
                            .await;
                    } else {
                        let _ = event_tx
                            .send(AppEvent::PrefixInitFinished {
                                prefix_name: prefix.name.clone(),
                                success: false,
                                error: Some(format!("wine boot exited with status: {}", status)),
                            })
                            .await;
                    }
                }
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::PrefixInitFinished {
                            prefix_name: prefix.name.clone(),
                            success: false,
                            error: Some(format!("Failed to wait for process: {}", e)),
                        })
                        .await;
                }
            },
            Err(e) => {
                let _ = event_tx
                    .send(AppEvent::PrefixInitFinished {
                        prefix_name: prefix.name.clone(),
                        success: false,
                        error: Some(format!("Failed to spawn wine boot: {}", e)),
                    })
                    .await;
            }
        }
    });
}
