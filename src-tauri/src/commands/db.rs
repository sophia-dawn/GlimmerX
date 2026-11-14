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
    mcp: State<'_, crate::mcp::McpRuntime>,
    state: State<'_, AppState>,
) -> Result<DbInfo, String> {
    let path_buf = PathBuf::from(&path);

    // Ensure parent directory exists
    if let Some(parent) = path_buf.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            eprintln!("[glimmerx] internal error: {e}");
            crate::errors::message("errors.databaseError")
        })?;
    }

    let db = Database::create(&path_buf, &password).map_err(crate::errors::from_error)?;

    // Store creation time in settings
    let now = crate::utils::time::now_rfc3339();
    {
        let conn = db.get_conn().map_err(crate::errors::from_error)?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            ("db_created_at", &now),
        )
        .map_err(|e| {
            eprintln!("[glimmerx] internal error: {e}");
            crate::errors::message("errors.databaseError")
        })?;
    }

    // Add to recent databases
    let mut recent_dbs = state.recent_dbs.lock().map_err(crate::errors::from_error)?;
    recent_dbs
        .add(&path, &path_label(&path_buf))
        .map_err(crate::errors::from_error)?;

    // Store in app state
    {
        let mut db_state = state.database.lock().map_err(crate::errors::from_error)?;
        *db_state = Some(db);
    }

    // The MCP server's settings (token included) live in the encrypted db,
    // so it materializes only once a db is available. Must run after the
    // state guard is released — `sync` locks it too.
    crate::mcp::sync_from(&mcp);

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
    mcp: State<'_, crate::mcp::McpRuntime>,
    state: State<'_, AppState>,
) -> Result<DbInfo, String> {
    let path_buf = PathBuf::from(&path);

    let db = Database::open(&path_buf, &password).map_err(crate::errors::from_error)?;

    // Read creation time from settings
    let created_at = {
        let conn = db.get_conn().map_err(crate::errors::from_error)?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'db_created_at'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "unknown".to_string())
    };

    // Add to recent databases
    let mut recent_dbs = state.recent_dbs.lock().map_err(crate::errors::from_error)?;
    recent_dbs
        .add(&path, &path_label(&path_buf))
        .map_err(crate::errors::from_error)?;

    // Store in app state
    {
        let mut db_state = state.database.lock().map_err(crate::errors::from_error)?;
        *db_state = Some(db);
    }

    crate::mcp::sync_from(&mcp);

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
    let db_state = state.database.lock().map_err(crate::errors::from_error)?;
    let db = db_state
        .as_ref()
        .ok_or(crate::errors::message("errors.databaseLocked").to_string())?;

    db.change_password_with_verification(&old_password, &new_password)
        .map_err(|e| match e {
            AppError::InvalidPassword => crate::errors::message("errors.invalidPassword"),
            other => other.to_string(),
        })
}

/// Check if a database file exists at the given path.
///
/// Restricted to `*.db` on purpose: the webview only ever asks this about a
/// ledger it is about to open, and an unrestricted version is a filesystem
/// oracle that keeps working while the ledger is locked.
#[tauri::command]
pub async fn db_check_exists(path: String) -> Result<bool, String> {
    let candidate = PathBuf::from(&path);
    let is_ledger_file = candidate
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("db"));
    if !is_ledger_file {
        return Err(crate::errors::message("errors.invalidDatabasePath").to_string());
    }
    Ok(candidate.is_file())
}

/// Check if any existing database file is known in recent databases.
#[tauri::command]
pub async fn db_check_any_exists(state: State<'_, AppState>) -> Result<bool, String> {
    let recent_dbs = state.recent_dbs.lock().map_err(crate::errors::from_error)?;
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
    let recent_dbs = state.recent_dbs.lock().map_err(crate::errors::from_error)?;
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
    let mut recent_dbs = state.recent_dbs.lock().map_err(crate::errors::from_error)?;
    recent_dbs.remove(&path).map_err(crate::errors::from_error)
}

/// Lock the database (drop the connection from state).
#[tauri::command]
pub async fn db_lock(state: State<'_, AppState>) -> Result<(), String> {
    let mut db_state = state.database.lock().map_err(crate::errors::from_error)?;
    // Explicit checkpoint before dropping (double insurance with Drop trait)
    if let Some(db) = db_state.as_ref() {
        let conn = db.get_conn().map_err(crate::errors::from_error)?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| {
                eprintln!("[glimmerx] internal error: {e}");
                crate::errors::message("errors.databaseError")
            })?;
        // `PRAGMA optimize` is meant to run when a connection closes: it only
        // analyses tables whose statistics have drifted enough to matter, so it
        // costs a few milliseconds and keeps report queries on the good plan as
        // the ledger grows. Best-effort — a lock that fails on this would leave
        // the key alive, which is far worse than a stale query plan.
        if let Err(e) = conn.execute_batch("PRAGMA optimize;") {
            eprintln!("[glimmerx] PRAGMA optimize failed: {e}");
        }
    }

    // Clear state, triggering Drop (which will also attempt checkpoint)
    *db_state = None;
    Ok(())
}

#[tauri::command]
pub async fn db_is_unlocked(state: State<'_, AppState>) -> Result<bool, String> {
    let db_state = state.database.lock().map_err(crate::errors::from_error)?;
    let result = db_state.is_some();
    Ok(result)
}

#[tauri::command]
pub async fn db_ping(state: State<'_, AppState>) -> Result<bool, String> {
    let db_state = state.database.lock().map_err(crate::errors::from_error)?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| crate::errors::message("errors.databaseLocked").to_string())?;

    let conn = db.get_conn().map_err(crate::errors::from_error)?;
    conn.query_row("SELECT 1", [], |_row| Ok(true))
        .map_err(|e| {
            eprintln!("[glimmerx] internal error: {e}");
            crate::errors::message("errors.databaseError")
        })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RecentDbs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn mock_app(
        state: AppState,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(state)
            // db_create/db_unlock restart the MCP server through this state.
            .manage(crate::mcp::McpRuntime::default())
            .invoke_handler(tauri::generate_handler![
                db_create,
                db_unlock,
                db_change_password,
                db_check_exists,
                db_check_any_exists,
                db_list_recent,
                db_remove_recent,
                db_lock,
                db_is_unlocked,
                db_ping,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        (app, webview)
    }

    fn invoke(
        webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        cmd: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        let req = tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        tauri::test::get_ipc_response(webview, req)
            .map(|b| b.deserialize::<serde_json::Value>().unwrap())
    }

    fn empty_state(recent_dbs: RecentDbs) -> AppState {
        AppState {
            database: Mutex::new(None),
            recent_dbs: Mutex::new(recent_dbs),
        }
    }

    fn test_recent() -> RecentDbs {
        RecentDbs::load_from_path(&TempDir::new().unwrap().path().join("recent_dbs.json")).unwrap()
    }

    #[test]
    fn test_db_create_unlock_lock_flow() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db").to_string_lossy().to_string();
        let (_app, webview) = mock_app(empty_state(test_recent()));

        let created = invoke(
            &webview,
            "db_create",
            serde_json::json!({ "path": db_path, "password": "secret" }),
        )
        .expect("db_create should succeed");
        assert_eq!(created["path"], serde_json::json!(db_path));

        let unlocked: bool = serde_json::from_value(
            invoke(&webview, "db_is_unlocked", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(unlocked);
        let ping: bool =
            serde_json::from_value(invoke(&webview, "db_ping", serde_json::json!({})).unwrap())
                .unwrap();
        assert!(ping);

        let exists: bool = serde_json::from_value(
            invoke(
                &webview,
                "db_check_exists",
                serde_json::json!({ "path": db_path }),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(exists);

        // Anything that is not a ledger file is refused, so this cannot be used
        // to probe the filesystem while the database is locked.
        assert!(invoke(
            &webview,
            "db_check_exists",
            serde_json::json!({ "path": "Cargo.toml" }),
        )
        .is_err());

        let recents: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "db_list_recent", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0]["path"], serde_json::json!(db_path));

        invoke(
            &webview,
            "db_remove_recent",
            serde_json::json!({ "path": db_path }),
        )
        .unwrap();
        let recents: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "db_list_recent", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(recents.is_empty());

        invoke(&webview, "db_lock", serde_json::json!({})).unwrap();
        let unlocked: bool = serde_json::from_value(
            invoke(&webview, "db_is_unlocked", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(!unlocked);
    }

    #[test]
    fn test_db_unlock_wrong_password_fails() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db").to_string_lossy().to_string();
        let (_app, webview) = mock_app(empty_state(test_recent()));

        invoke(
            &webview,
            "db_create",
            serde_json::json!({ "path": db_path, "password": "secret" }),
        )
        .unwrap();
        invoke(&webview, "db_lock", serde_json::json!({})).unwrap();

        let err = invoke(
            &webview,
            "db_unlock",
            serde_json::json!({ "path": db_path, "password": "wrong" }),
        )
        .expect_err("wrong password must fail");
        assert_eq!(err.as_str().unwrap(), "密码错误或数据库已损坏");

        invoke(
            &webview,
            "db_unlock",
            serde_json::json!({ "path": db_path, "password": "secret" }),
        )
        .unwrap();
    }

    #[test]
    fn test_db_change_password_flow() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db").to_string_lossy().to_string();
        let (_app, webview) = mock_app(empty_state(test_recent()));

        invoke(
            &webview,
            "db_create",
            serde_json::json!({ "path": db_path, "password": "old-pass" }),
        )
        .unwrap();

        invoke(
            &webview,
            "db_change_password",
            serde_json::json!({ "oldPassword": "old-pass", "newPassword": "new-pass" }),
        )
        .unwrap();

        invoke(&webview, "db_lock", serde_json::json!({})).unwrap();

        let err = invoke(
            &webview,
            "db_unlock",
            serde_json::json!({ "path": db_path, "password": "old-pass" }),
        )
        .expect_err("old password must fail after rekey");
        assert_eq!(err.as_str().unwrap(), "密码错误或数据库已损坏");

        invoke(
            &webview,
            "db_unlock",
            serde_json::json!({ "path": db_path, "password": "new-pass" }),
        )
        .unwrap();
    }

    #[test]
    fn test_db_check_any_exists() {
        let (_app, webview) = mock_app(empty_state(test_recent()));

        let any: bool = serde_json::from_value(
            invoke(&webview, "db_check_any_exists", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(!any);

        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db").to_string_lossy().to_string();
        invoke(
            &webview,
            "db_create",
            serde_json::json!({ "path": db_path, "password": "secret" }),
        )
        .unwrap();
        let any: bool = serde_json::from_value(
            invoke(&webview, "db_check_any_exists", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(any);
    }

    #[test]
    fn test_db_change_password_wrong_old_password() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db").to_string_lossy().to_string();
        let (_app, webview) = mock_app(empty_state(test_recent()));

        invoke(
            &webview,
            "db_create",
            serde_json::json!({ "path": db_path, "password": "secret" }),
        )
        .unwrap();

        let err = invoke(
            &webview,
            "db_change_password",
            serde_json::json!({ "oldPassword": "wrong", "newPassword": "new-pass" }),
        )
        .expect_err("changing password with wrong old password must fail");
        assert_eq!(err.as_str().unwrap(), "密码错误或数据库已损坏");
    }

    #[test]
    fn test_db_ping_locked_fails() {
        let (_app, webview) = mock_app(empty_state(test_recent()));
        let err = invoke(&webview, "db_ping", serde_json::json!({}))
            .expect_err("ping without unlocked database must fail");
        assert!(err.as_str().unwrap().contains("数据库未解锁"));
    }
}
