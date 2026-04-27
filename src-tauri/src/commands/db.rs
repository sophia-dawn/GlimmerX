use serde::Serialize;
use tauri::State;

use crate::db::{AppError, Database};
use crate::AppState;

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct DbInfo {
    pub path: String,
    pub label: String,
    pub created_at: String,
}

/// A recent database entry (mirrors `db::RecentDb`).
#[derive(Serialize, Clone)]
pub struct RecentDbResponse {
    pub path: String,
    pub label: String,
    pub last_opened: String,
    pub exists: bool,
}

// ---------------------------------------------------------------------------
// Helper: extract label from a file path
// ---------------------------------------------------------------------------

fn path_label(path: &std::path::Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string())
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

/// Create a new encrypted database at a user-chosen path and initialize the schema.
#[tauri::command]
pub async fn db_create(
    password: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<DbInfo, String> {
    let path_buf = PathBuf::from(&path);

    // Ensure parent directory exists
    if let Some(parent) = path_buf.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let db = Database::create(&path_buf, &password).map_err(|e| e.to_string())?;

    // Store creation time in settings
    let now = crate::utils::time::now_rfc3339();
    {
        let conn = db.get_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            ("db_created_at", &now),
        )
        .map_err(|e| format!("Failed to save creation time: {}", e))?;
    }

    // Add to recent databases
    let mut recent_dbs = state.recent_dbs.lock().map_err(|e| e.to_string())?;
    recent_dbs
        .add(&path, &path_label(&path_buf))
        .map_err(|e| e.to_string())?;

    // Store in app state
    let mut db_state = state.database.lock().map_err(|e| e.to_string())?;
    *db_state = Some(db);

    Ok(DbInfo {
        path: path_buf.to_string_lossy().to_string(),
        label: path_label(&path_buf),
        created_at: now,
    })
}

/// Unlock an existing database at the given path with the given password.
#[tauri::command]
pub async fn db_unlock(
    password: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<DbInfo, String> {
    let path_buf = PathBuf::from(&path);

    let db = Database::open(&path_buf, &password).map_err(|e| e.to_string())?;

    // Read creation time from settings
    let created_at = {
        let conn = db.get_conn().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'db_created_at'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "unknown".to_string())
    };

    // Add to recent databases
    let mut recent_dbs = state.recent_dbs.lock().map_err(|e| e.to_string())?;
    recent_dbs
        .add(&path, &path_label(&path_buf))
        .map_err(|e| e.to_string())?;

    // Store in app state
    let mut db_state = state.database.lock().map_err(|e| e.to_string())?;
    *db_state = Some(db);

    Ok(DbInfo {
        path: path_buf.to_string_lossy().to_string(),
        label: path_label(&path_buf),
        created_at,
    })
}

/// Change the database password.
#[tauri::command]
pub async fn db_change_password(
    old_password: String,
    new_password: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;

    db.change_password_with_verification(&old_password, &new_password)
        .map_err(|e| match e {
            AppError::InvalidPassword => "errors.invalidPassword".to_string(),
            other => other.to_string(),
        })
}

/// Check if a database file exists at the given path.
#[tauri::command]
pub async fn db_check_exists(path: String) -> Result<bool, String> {
    Ok(PathBuf::from(path).exists())
}

/// Check if any existing database file is known in recent databases.
#[tauri::command]
pub async fn db_check_any_exists(state: State<'_, AppState>) -> Result<bool, String> {
    let recent_dbs = state.recent_dbs.lock().map_err(|e| e.to_string())?;
    for entry in recent_dbs.list() {
        if PathBuf::from(&entry.path).exists() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// List recent databases.
#[tauri::command]
pub async fn db_list_recent(state: State<'_, AppState>) -> Result<Vec<RecentDbResponse>, String> {
    let recent_dbs = state.recent_dbs.lock().map_err(|e| e.to_string())?;
    Ok(recent_dbs
        .list()
        .iter()
        .map(|e| RecentDbResponse {
            path: e.path.clone(),
            label: e.label.clone(),
            last_opened: e.last_opened.clone(),
            exists: PathBuf::from(&e.path).exists(),
        })
        .collect())
}

/// Remove a database from recent list.
#[tauri::command]
pub async fn db_remove_recent(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut recent_dbs = state.recent_dbs.lock().map_err(|e| e.to_string())?;
    recent_dbs.remove(&path).map_err(|e| e.to_string())
}

/// Lock the database (drop the connection from state).
#[tauri::command]
pub async fn db_lock(state: State<'_, AppState>) -> Result<(), String> {
    eprintln!("[db_lock] start");
    let mut db_state = state.database.lock().map_err(|e| {
        eprintln!("[db_lock] mutex lock failed: {}", e);
        e.to_string()
    })?;

    eprintln!(
        "[db_lock] current state: {}",
        if db_state.is_some() {
            "has database"
        } else {
            "empty"
        }
    );

    // Explicit checkpoint before dropping (double insurance with Drop trait)
    if let Some(db) = db_state.as_ref() {
        let conn = db.get_conn().map_err(|e| {
            eprintln!("[db_lock] get_conn failed: {}", e);
            e.to_string()
        })?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| {
                eprintln!("[db_lock] checkpoint failed: {}", e);
                format!("Checkpoint failed: {}", e)
            })?;
        eprintln!("[db_lock] explicit checkpoint completed");
    }

    // Clear state, triggering Drop (which will also attempt checkpoint)
    *db_state = None;
    eprintln!("[db_lock] state cleared");
    Ok(())
}

#[tauri::command]
pub async fn db_is_unlocked(state: State<'_, AppState>) -> Result<bool, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let result = db_state.is_some();
    eprintln!("[db_is_unlocked] result: {}", result);
    Ok(result)
}

#[tauri::command]
pub async fn db_ping(state: State<'_, AppState>) -> Result<bool, String> {
    eprintln!("[db_ping] start");
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state.as_ref().ok_or_else(|| {
        eprintln!("[db_ping] database is not unlocked");
        "Database is not unlocked. Please unlock it first.".to_string()
    })?;

    let conn = db.get_conn().map_err(|e| {
        eprintln!("[db_ping] get_conn failed: {}", e);
        e.to_string()
    })?;
    conn.query_row("SELECT 1", [], |_row| Ok(true))
        .map_err(|e| {
            eprintln!("[db_ping] query failed: {}", e);
            format!("Database ping failed: {}", e)
        })?;

    eprintln!("[db_ping] success");
    Ok(true)
}
