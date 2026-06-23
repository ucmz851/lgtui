use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum GameStatus {
    #[default]
    Ready,
    Running {
        pid: u32,
        since: u64,
    },
    Error(String),
}

impl std::fmt::Display for GameStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameStatus::Ready => write!(f, "Ready"),
            GameStatus::Running { pid, .. } => write!(f, "Running (PID: {})", pid),
            GameStatus::Error(err) => write!(f, "Error: {}", err),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub exec_path: String,
    pub args: Vec<String>,
    pub wineprefix: Option<String>,
    pub runner_id: Option<String>,
    pub playtime_secs: u64,
    pub dxvk: bool,
    pub vkd3d: bool,
    pub mangohud: bool,
    pub gamemode: bool,
    pub is_installer: bool,
    #[serde(skip)]
    pub status: GameStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub installed: bool,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_wineprefix: String,
    pub runner_download_dir: String,
    pub games: Vec<Game>,
    pub runners: Vec<Runner>,
}

impl Default for Config {
    fn default() -> Self {
        let base_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local/share/lgtui"))
            .unwrap_or_else(|_| PathBuf::from("./.lgtui"));

        let default_prefix = base_dir
            .join("prefixes/default")
            .to_string_lossy()
            .to_string();
        let download_dir = base_dir.join("runners").to_string_lossy().to_string();

        let games = Vec::new();

        let runners = vec![
            Runner {
                id: "proton-ge-9-5".to_string(),
                name: "Proton-GE".to_string(),
                version: "GE-Proton9-5".to_string(),
                path: base_dir
                    .join("runners/GE-Proton9-5/proton")
                    .to_string_lossy()
                    .to_string(),
                installed: true,
                download_url: None,
            },
            Runner {
                id: "wine-staging-9-10".to_string(),
                name: "Wine Staging".to_string(),
                version: "9.10".to_string(),
                path: "/usr/bin/wine".to_string(),
                installed: true,
                download_url: None,
            },
            Runner {
                id: "proton-ge-8-25".to_string(),
                name: "Proton-GE".to_string(),
                version: "GE-Proton8-25".to_string(),
                path: base_dir
                    .join("runners/GE-Proton8-25/proton")
                    .to_string_lossy()
                    .to_string(),
                installed: false,
                download_url: None,
            },
            Runner {
                id: "proton-9-beta".to_string(),
                name: "Proton (Beta)".to_string(),
                version: "Proton 9.0 (Beta 3)".to_string(),
                path: base_dir
                    .join("runners/Proton-9.0-Beta/proton")
                    .to_string_lossy()
                    .to_string(),
                installed: false,
                download_url: None,
            },
        ];

        Config {
            default_wineprefix: default_prefix,
            runner_download_dir: download_dir,
            games,
            runners,
        }
    }
}

impl Game {
    pub fn get_toml_path(game_id: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config/lgui/games")
            .join(format!("{}.toml", game_id))
    }

    pub fn save_toml(&self) -> Result<(), std::io::Error> {
        let path = Self::get_toml_path(&self.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::other(format!("Failed to serialize game to TOML: {}", e))
        })?;
        fs::write(path, content)?;

        // Also save to SQLite database
        let conn = crate::database::init_db().map_err(|e| std::io::Error::other(e.to_string()))?;
        crate::database::save_game(&conn, self)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }

    pub fn delete_toml(&self) -> Result<(), std::io::Error> {
        let path = Self::get_toml_path(&self.id);
        if path.exists() {
            let _ = fs::remove_file(path);
        }

        // Also delete from SQLite database
        let conn = crate::database::init_db().map_err(|e| std::io::Error::other(e.to_string()))?;
        crate::database::delete_game(&conn, &self.id)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }
}

impl Config {
    pub fn load_or_create() -> Self {
        let conn = match crate::database::init_db() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize database: {}", e);
                return Config::default();
            }
        };

        // Check if database is empty/needs migration
        let needs_migration = crate::database::get_setting(&conn, "default_wineprefix")
            .ok()
            .flatten()
            .is_none();

        if needs_migration {
            let path = Path::new("lgtui_config.json");
            let config = if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    serde_json::from_str::<Config>(&content).unwrap_or_else(|_| Config::default())
                } else {
                    Config::default()
                }
            } else {
                Config::default()
            };

            // Migrate default prefix and runner download cache settings
            let _ = crate::database::save_setting(
                &conn,
                "default_wineprefix",
                &config.default_wineprefix,
            );
            let _ = crate::database::save_setting(
                &conn,
                "runner_download_dir",
                &config.runner_download_dir,
            );

            // Migrate games
            for game in &config.games {
                let _ = crate::database::save_game(&conn, game);
            }

            // Migrate runners
            for runner in &config.runners {
                let _ = crate::database::save_runner(&conn, runner);
            }

            // Migrate prefixes from prefixes.toml if it exists
            let prefix_config_path = crate::prefix::PrefixConfig::get_config_path();
            if prefix_config_path.exists() {
                if let Ok(content) = fs::read_to_string(&prefix_config_path) {
                    if let Ok(prefix_config) =
                        toml::from_str::<crate::prefix::PrefixConfig>(&content)
                    {
                        for prefix in &prefix_config.prefixes {
                            let _ = crate::database::save_prefix(&conn, prefix);
                        }
                    }
                }
            }

            // Clean up old JSON config files
            let _ = fs::remove_file("lgtui_config.json");
        }

        // Load settings from database
        let default_wineprefix = crate::database::get_setting(&conn, "default_wineprefix")
            .ok()
            .flatten()
            .unwrap_or_else(|| Config::default().default_wineprefix);

        let runner_download_dir = crate::database::get_setting(&conn, "runner_download_dir")
            .ok()
            .flatten()
            .unwrap_or_else(|| Config::default().runner_download_dir);

        let games = crate::database::get_all_games(&conn).unwrap_or_default();
        let runners = crate::database::get_all_runners(&conn).unwrap_or_default();

        Config {
            default_wineprefix,
            runner_download_dir,
            games,
            runners,
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let conn = crate::database::init_db().map_err(|e| std::io::Error::other(e.to_string()))?;
        crate::database::save_setting(&conn, "default_wineprefix", &self.default_wineprefix)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        crate::database::save_setting(&conn, "runner_download_dir", &self.runner_download_dir)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        for game in &self.games {
            crate::database::save_game(&conn, game)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }

        for runner in &self.runners {
            crate::database::save_runner(&conn, runner)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }

        Ok(())
    }
}
