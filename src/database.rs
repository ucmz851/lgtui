use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::game::{Game, GameStatus, Runner};
use crate::prefix::WinePrefix;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameStats {
    pub game_id: String,
    pub avg_fps: u32,
    pub max_fps: u32,
    pub last_played: Option<u64>, // Unix timestamp in seconds
    pub session_count: u32,
}

pub fn get_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home)
        .join(".local/share/lgtui")
        .join("lgtui.db")
}

pub fn init_db() -> Result<Connection, rusqlite::Error> {
    let path = get_db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;

    // Create games table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            exec_path TEXT NOT NULL,
            args TEXT,
            wineprefix TEXT,
            runner_id TEXT,
            playtime_secs INTEGER DEFAULT 0,
            dxvk INTEGER DEFAULT 0,
            vkd3d INTEGER DEFAULT 0,
            mangohud INTEGER DEFAULT 0,
            gamemode INTEGER DEFAULT 0,
            is_installer INTEGER DEFAULT 0
        )",
        [],
    )?;

    // Create game_statistics table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_statistics (
            game_id TEXT PRIMARY KEY,
            avg_fps INTEGER DEFAULT 0,
            max_fps INTEGER DEFAULT 0,
            last_played INTEGER,
            session_count INTEGER DEFAULT 0,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create prefixes table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS prefixes (
            name TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            architecture TEXT NOT NULL
        )",
        [],
    )?;

    // Create settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    // Create runners table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS runners (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            path TEXT NOT NULL,
            installed INTEGER DEFAULT 0,
            download_url TEXT
        )",
        [],
    )?;

    Ok(conn)
}

pub fn save_setting(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        let val: String = row.get(0)?;
        Ok(Some(val))
    } else {
        Ok(None)
    }
}

pub fn save_runner(conn: &Connection, runner: &Runner) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO runners (id, name, version, path, installed, download_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            runner.id,
            runner.name,
            runner.version,
            runner.path,
            if runner.installed { 1 } else { 0 },
            runner.download_url
        ],
    )?;
    Ok(())
}

pub fn get_all_runners(conn: &Connection) -> Result<Vec<Runner>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT id, name, version, path, installed, download_url FROM runners")?;
    let runner_iter = stmt.query_map([], |row: &Row| {
        Ok(Runner {
            id: row.get(0)?,
            name: row.get(1)?,
            version: row.get(2)?,
            path: row.get(3)?,
            installed: row.get::<_, i32>(4)? != 0,
            download_url: row.get(5)?,
        })
    })?;

    let mut runners = Vec::new();
    for runner in runner_iter {
        runners.push(runner?);
    }
    Ok(runners)
}

pub fn save_game(conn: &Connection, game: &Game) -> Result<(), rusqlite::Error> {
    let args_json = serde_json::to_string(&game.args).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO games (
            id, name, exec_path, args, wineprefix, runner_id, playtime_secs,
            dxvk, vkd3d, mangohud, gamemode, is_installer
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            game.id,
            game.name,
            game.exec_path,
            args_json,
            game.wineprefix,
            game.runner_id,
            game.playtime_secs,
            if game.dxvk { 1 } else { 0 },
            if game.vkd3d { 1 } else { 0 },
            if game.mangohud { 1 } else { 0 },
            if game.gamemode { 1 } else { 0 },
            if game.is_installer { 1 } else { 0 }
        ],
    )?;

    // Also ensure stats row exists
    conn.execute(
        "INSERT OR IGNORE INTO game_statistics (game_id, avg_fps, max_fps, last_played, session_count)
         VALUES (?1, 0, 0, NULL, 0)",
        params![game.id],
    )?;

    Ok(())
}

pub fn get_all_games(conn: &Connection) -> Result<Vec<Game>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, exec_path, args, wineprefix, runner_id, playtime_secs,
                dxvk, vkd3d, mangohud, gamemode, is_installer FROM games",
    )?;

    let game_iter = stmt.query_map([], |row: &Row| {
        let args_json: Option<String> = row.get(3)?;
        let args: Vec<String> = args_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Game {
            id: row.get(0)?,
            name: row.get(1)?,
            exec_path: row.get(2)?,
            args,
            wineprefix: row.get(4)?,
            runner_id: row.get(5)?,
            playtime_secs: row.get(6)?,
            dxvk: row.get::<_, i32>(7)? != 0,
            vkd3d: row.get::<_, i32>(8)? != 0,
            mangohud: row.get::<_, i32>(9)? != 0,
            gamemode: row.get::<_, i32>(10)? != 0,
            is_installer: row.get::<_, i32>(11)? != 0,
            status: GameStatus::Ready,
        })
    })?;

    let mut games = Vec::new();
    for game in game_iter {
        games.push(game?);
    }
    Ok(games)
}

pub fn delete_game(conn: &Connection, game_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM games WHERE id = ?1", params![game_id])?;
    conn.execute(
        "DELETE FROM game_statistics WHERE game_id = ?1",
        params![game_id],
    )?;
    Ok(())
}

pub fn save_prefix(conn: &Connection, prefix: &WinePrefix) -> Result<(), rusqlite::Error> {
    let path_str = prefix.path.to_string_lossy().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO prefixes (name, path, architecture) VALUES (?1, ?2, ?3)",
        params![prefix.name, path_str, prefix.architecture],
    )?;
    Ok(())
}

pub fn get_all_prefixes(conn: &Connection) -> Result<Vec<WinePrefix>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT name, path, architecture FROM prefixes")?;
    let prefix_iter = stmt.query_map([], |row: &Row| {
        let path_str: String = row.get(1)?;
        let path = PathBuf::from(path_str);

        // Determine status dynamically based on registry file presence
        let reg_file = path.join("user.reg");
        let status = if reg_file.exists() {
            "Ready".to_string()
        } else {
            "Not Initialized".to_string()
        };

        Ok(WinePrefix {
            name: row.get(0)?,
            path,
            architecture: row.get(2)?,
            status,
        })
    })?;

    let mut prefixes = Vec::new();
    for prefix in prefix_iter {
        prefixes.push(prefix?);
    }
    Ok(prefixes)
}

pub fn delete_prefix(conn: &Connection, name: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM prefixes WHERE name = ?1", params![name])?;
    Ok(())
}

pub fn get_game_stats(conn: &Connection, game_id: &str) -> Result<GameStats, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT game_id, avg_fps, max_fps, last_played, session_count FROM game_statistics WHERE game_id = ?1",
    )?;
    let mut rows = stmt.query(params![game_id])?;
    if let Some(row) = rows.next()? {
        Ok(GameStats {
            game_id: row.get(0)?,
            avg_fps: row.get(1)?,
            max_fps: row.get(2)?,
            last_played: row.get(3)?,
            session_count: row.get(4)?,
        })
    } else {
        Ok(GameStats {
            game_id: game_id.to_string(),
            avg_fps: 0,
            max_fps: 0,
            last_played: None,
            session_count: 0,
        })
    }
}

pub fn record_game_launch(conn: &Connection, game_id: &str) -> Result<(), rusqlite::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    conn.execute(
        "UPDATE game_statistics SET session_count = session_count + 1, last_played = ?2 WHERE game_id = ?1",
        params![game_id, now],
    )?;
    Ok(())
}

pub fn record_game_exit(
    conn: &Connection,
    game_id: &str,
    session_playtime: u64,
    avg_fps: u32,
    max_fps: u32,
) -> Result<(), rusqlite::Error> {
    // 1. Update playtime in games table
    conn.execute(
        "UPDATE games SET playtime_secs = playtime_secs + ?2 WHERE id = ?1",
        params![game_id, session_playtime],
    )?;

    // 2. Fetch current statistics to compute running average FPS
    let stats = get_game_stats(conn, game_id)?;
    let new_avg = if stats.session_count <= 1 || stats.avg_fps == 0 {
        avg_fps
    } else {
        // Simple running average calculation
        let total_launches = stats.session_count;
        ((stats.avg_fps * (total_launches - 1)) + avg_fps) / total_launches
    };

    let new_max = std::cmp::max(stats.max_fps, max_fps);

    // 3. Update game_statistics table
    conn.execute(
        "UPDATE game_statistics SET avg_fps = ?2, max_fps = ?3 WHERE game_id = ?1",
        params![game_id, new_avg, new_max],
    )?;
    Ok(())
}
