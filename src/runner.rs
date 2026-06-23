use tokio::sync::mpsc;

use crate::game::{Game, Runner};

#[derive(Debug)]
pub enum AppEvent {
    GameStarted {
        game_id: String,
        pid: u32,
    },
    GameFinished {
        game_id: String,
        exit_code: i32,
    },
    ReleasesFetched(Vec<Runner>),
    DownloadProgress(u8),
    ExtractionStarted,
    DownloadFinished {
        runner_id: String,
        path: String,
    },
    DownloadError(String),
    PrefixInitStarted {
        prefix_name: String,
    },
    PrefixInitFinished {
        prefix_name: String,
        success: bool,
        error: Option<String>,
    },
    ScriptStarted,
    ScriptLine(String),
    ScriptFinished(bool),
    OnboardingScanFinished(crate::onboarding::DiagnosticsResult),
    InstallerFinished {
        game_id: String,
        exec_path: String,
        wineprefix: String,
    },
}

pub struct RunnerManager;

impl RunnerManager {
    /// Launches a game in the background and monitors its lifecycle.
    pub fn launch(game: Game, runner: Option<Runner>, event_tx: mpsc::Sender<AppEvent>) {
        crate::process::launch_game_or_installer(game, runner, event_tx);
    }

    /// Spawns a background task to fetch latest Proton releases from GitHub.
    pub fn fetch_releases(event_tx: mpsc::Sender<AppEvent>) {
        tokio::spawn(async move {
            match Self::fetch_latest_proton_releases().await {
                Ok(runners) => {
                    let _ = event_tx.send(AppEvent::ReleasesFetched(runners)).await;
                }
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!(
                            "Failed to fetch releases: {}",
                            e
                        )))
                        .await;
                }
            }
        });
    }

    async fn fetch_latest_proton_releases(
    ) -> Result<Vec<Runner>, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .user_agent("lgtui-downloader/0.1.0")
            .build()?;

        // Use GitHub Releases Atom feed to bypass REST API rate limits
        let url = "https://github.com/GloriousEggroll/proton-ge-custom/releases.atom";
        let request = client.get(url);
        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(format!("GitHub returned status: {}", response.status()).into());
        }

        let response_text = response.text().await?;

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let steam_compat_dir =
            std::path::PathBuf::from(&home).join(".local/share/steam/compatibilitytools.d");

        let mut runners = Vec::new();
        let search_str = "https://github.com/GloriousEggroll/proton-ge-custom/releases/tag/";
        let mut start_idx = 0;

        while let Some(idx) = response_text[start_idx..].find(search_str) {
            let actual_idx = start_idx + idx;
            let tag_start = actual_idx + search_str.len();
            if let Some(end_offset) = response_text[tag_start..].find('"') {
                let tag_name = &response_text[tag_start..tag_start + end_offset];

                // Exclude any empty/invalid tags or duplicates in the feed
                if !tag_name.is_empty() && !runners.iter().any(|r: &Runner| r.version == tag_name) {
                    let id = tag_name.to_lowercase();
                    let dest_path = steam_compat_dir.join(tag_name);
                    let installed = dest_path.exists() && dest_path.join("proton").exists();
                    let download_url = format!(
                        "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/{}/{}.tar.gz",
                        tag_name, tag_name
                    );

                    runners.push(Runner {
                        id,
                        name: "Proton-GE".to_string(),
                        version: tag_name.to_string(),
                        path: dest_path.join("proton").to_string_lossy().to_string(),
                        installed,
                        download_url: Some(download_url),
                    });
                }
                start_idx = tag_start + end_offset;
            } else {
                break;
            }
        }

        Ok(runners)
    }

    /// Downloads the runner tarball in chunks, updates progress, and extracts it on complete.
    pub fn download_and_extract_runner(
        runner_id: String,
        url: String,
        dest_dir: std::path::PathBuf,
        event_tx: mpsc::Sender<AppEvent>,
    ) {
        tokio::spawn(async move {
            let client = match reqwest::Client::builder()
                .user_agent("lgtui-downloader/0.1.0")
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!(
                            "Failed to build HTTP client: {}",
                            e
                        )))
                        .await;
                    return;
                }
            };

            let response = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!("Request failed: {}", e)))
                        .await;
                    return;
                }
            };

            let total_size = response.content_length();

            // Create destination and temp directories
            if let Some(parent) = dest_dir.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!(
                            "Failed to create destination parent: {}",
                            e
                        )))
                        .await;
                    return;
                }
            }

            let temp_file_path = dest_dir.with_extension("tar.gz.part");
            if let Some(parent) = temp_file_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!(
                            "Failed to create temp directory: {}",
                            e
                        )))
                        .await;
                    return;
                }
            }

            let mut file = match tokio::fs::File::create(&temp_file_path).await {
                Ok(f) => f,
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!(
                            "Failed to create temp file: {}",
                            e
                        )))
                        .await;
                    return;
                }
            };

            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();

            use futures_util::StreamExt;
            use tokio::io::AsyncWriteExt;

            while let Some(item) = stream.next().await {
                let chunk = match item {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = event_tx
                            .send(AppEvent::DownloadError(format!(
                                "Error downloading chunk: {}",
                                e
                            )))
                            .await;
                        let _ = tokio::fs::remove_file(&temp_file_path).await;
                        return;
                    }
                };

                if let Err(e) = file.write_all(&chunk).await {
                    let _ = event_tx
                        .send(AppEvent::DownloadError(format!(
                            "Failed to write to file: {}",
                            e
                        )))
                        .await;
                    let _ = tokio::fs::remove_file(&temp_file_path).await;
                    return;
                }

                downloaded += chunk.len() as u64;

                if let Some(total) = total_size {
                    let percentage = ((downloaded * 100) / total) as u8;
                    let _ = event_tx.send(AppEvent::DownloadProgress(percentage)).await;
                } else {
                    let _ = event_tx.send(AppEvent::DownloadProgress(0)).await;
                }
            }

            if let Err(e) = file.flush().await {
                let _ = event_tx
                    .send(AppEvent::DownloadError(format!(
                        "Failed to flush file: {}",
                        e
                    )))
                    .await;
                let _ = tokio::fs::remove_file(&temp_file_path).await;
                return;
            }

            // Finished download, start extraction
            let _ = event_tx.send(AppEvent::ExtractionStarted).await;

            // Perform extraction synchronously in a blocking thread pool
            let temp_file_path_clone = temp_file_path.clone();
            let dest_dir_clone = dest_dir.clone();
            let runner_id_clone = runner_id.clone();
            let event_tx_clone = event_tx.clone();

            tokio::task::spawn_blocking(move || {
                let file = match std::fs::File::open(&temp_file_path_clone) {
                    Ok(f) => f,
                    Err(e) => {
                        let _ = event_tx_clone.blocking_send(AppEvent::DownloadError(format!(
                            "Failed to open downloaded file: {}",
                            e
                        )));
                        let _ = std::fs::remove_file(&temp_file_path_clone);
                        return;
                    }
                };

                let tar_gz = flate2::read::GzDecoder::new(file);
                let mut archive = tar::Archive::new(tar_gz);

                let unpack_dest = match dest_dir_clone.parent() {
                    Some(p) => p,
                    None => &dest_dir_clone,
                };

                if let Err(e) = std::fs::create_dir_all(unpack_dest) {
                    let _ = event_tx_clone.blocking_send(AppEvent::DownloadError(format!(
                        "Failed to create destination folder: {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&temp_file_path_clone);
                    return;
                }

                if let Err(e) = archive.unpack(unpack_dest) {
                    let _ = event_tx_clone.blocking_send(AppEvent::DownloadError(format!(
                        "Extraction failed: {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&temp_file_path_clone);
                    return;
                }

                // Cleanup temp file
                let _ = std::fs::remove_file(&temp_file_path_clone);

                // Success!
                let _ = event_tx_clone.blocking_send(AppEvent::DownloadFinished {
                    runner_id: runner_id_clone,
                    path: dest_dir_clone.join("proton").to_string_lossy().to_string(),
                });
            })
            .await
            .unwrap_or_else(|e| {
                let _ = event_tx.blocking_send(AppEvent::DownloadError(format!(
                    "Blocking task panicked: {}",
                    e
                )));
            });
        });
    }

    /// Safely executes an external sh script and streams stdout/stderr lines.
    pub fn run_dependency_script(
        script_path: std::path::PathBuf,
        event_tx: mpsc::Sender<AppEvent>,
    ) {
        tokio::spawn(async move {
            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg(&script_path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = event_tx
                        .send(AppEvent::ScriptLine(format!(
                            "Error spawning script: {}",
                            e
                        )))
                        .await;
                    let _ = event_tx.send(AppEvent::ScriptFinished(false)).await;
                    return;
                }
            };

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            let _ = event_tx.send(AppEvent::ScriptStarted).await;

            loop {
                tokio::select! {
                    line = stdout_reader.next_line() => {
                        if let Ok(Some(l)) = line {
                            let _ = event_tx.send(AppEvent::ScriptLine(l)).await;
                        }
                    }
                    line = stderr_reader.next_line() => {
                        if let Ok(Some(l)) = line {
                            let _ = event_tx.send(AppEvent::ScriptLine(format!("ERROR: {}", l))).await;
                        }
                    }
                    status = child.wait() => {
                        let success = match status {
                            Ok(s) => s.success(),
                            _ => false,
                        };
                        let _ = event_tx.send(AppEvent::ScriptFinished(success)).await;
                        break;
                    }
                }
            }
        });
    }
}
