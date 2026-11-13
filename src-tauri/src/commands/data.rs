//! Data management commands.
//!
//! Only database backup lives here. Transaction import/export was removed on
//! 2026-11-13 (see `docs/design/21-double-entry-upgrade.md` §7): the product is
//! single-currency and hard-delete, so a CSV round-trip no longer carries enough
//! meaning to justify the maintenance cost. The database backup stays because it
//! is the safety net before migrations.

use std::path::{Path, PathBuf};
use tauri::State;

use crate::AppState;

/// Backup the current database to a specified path.
///
/// Performs a WAL checkpoint first, then copies the database file.
/// The backup retains the original encryption password.
#[tauri::command]
pub async fn db_backup(backup_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_state = state.database.lock().map_err(crate::errors::from_error)?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| crate::errors::message("errors.databaseLocked"))?;

    let conn = db.get_conn().map_err(crate::errors::from_error)?;
    backup_database(&conn, &db.path, Path::new(&backup_path))
}

/// Copy the database file to `backup_path` after folding the WAL into the main
/// file.
///
/// Kept separate from the command so the path rules below can be tested without
/// a `State<AppState>`.
fn backup_database(
    conn: &rusqlite::Connection,
    db_path: &Path,
    backup_path: &Path,
) -> Result<(), String> {
    // Refusing this case is not cosmetic: the remove-then-rename sequence below
    // would unlink the file the open connection is still writing to (on Unix),
    // leaving the app pointing at a deleted inode whose later writes vanish.
    if backup_path.exists() && is_same_file(db_path, backup_path) {
        eprintln!(
            "[glimmerx] backup target is the live database: {}",
            backup_path.display()
        );
        return Err(crate::errors::message("errors.backupSamePath"));
    }

    if let Some(parent) = backup_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            eprintln!(
                "[glimmerx] backup directory does not exist: {}",
                parent.display()
            );
            return Err(crate::errors::message("errors.backupFailed"));
        }
    }

    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| {
            eprintln!("[glimmerx] internal error: {e}");
            crate::errors::message("errors.databaseError")
        })?;

    // Copy to a temporary sibling first, then rename into place so an
    // interrupted backup never leaves a truncated file at the destination.
    let tmp_path = temporary_sibling(backup_path);
    let outcome = copy_then_replace(db_path, &tmp_path, backup_path);
    if let Err(e) = outcome {
        // Never leave a partial backup behind.
        let _ = std::fs::remove_file(&tmp_path);
        eprintln!("[glimmerx] backup failed: {e}");
        return Err(crate::errors::message("errors.backupFailed"));
    }
    Ok(())
}

fn temporary_sibling(backup_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.tmp-backup", backup_path.display()))
}

fn copy_then_replace(src: &Path, tmp_path: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::copy(src, tmp_path)?;
    if dest.exists() {
        std::fs::remove_file(dest)?;
    }
    std::fs::rename(tmp_path, dest)
}

fn is_same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn mock_app_with_db(
        db: Database,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(AppState {
                database: Mutex::new(Some(db)),
                recent_dbs: Mutex::new(RecentDbs::empty()),
            })
            .invoke_handler(tauri::generate_handler![
                db_backup,
                crate::commands::db::db_lock,
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

    #[test]
    fn test_db_backup_creates_usable_file() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let backup_path = dir.path().join("backup.db").to_string_lossy().to_string();
        invoke(
            &webview,
            "db_backup",
            serde_json::json!({ "backupPath": backup_path }),
        )
        .expect("backup should succeed");

        assert!(Path::new(&backup_path).exists());
        // No temporary file left behind
        assert!(!Path::new(&format!("{}.tmp-backup", backup_path)).exists());
        // Backup can be opened with the same password
        Database::open(&std::path::PathBuf::from(&backup_path), "secret")
            .expect("backup must be a valid encrypted database");
    }

    #[test]
    fn test_db_backup_after_lock_is_rejected() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let backup_path = dir.path().join("backup.db").to_string_lossy().to_string();
        invoke(
            &webview,
            "db_backup",
            serde_json::json!({ "backupPath": &backup_path }),
        )
        .unwrap();
        std::fs::remove_file(&backup_path).unwrap();

        invoke(&webview, "db_lock", serde_json::json!({})).expect("lock should succeed");

        // Nothing to back up once the key is gone and the connection dropped.
        let err = invoke(
            &webview,
            "db_backup",
            serde_json::json!({ "backupPath": &backup_path }),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("解锁") || err.to_string().to_lowercase().contains("lock"),
            "unexpected message: {err}"
        );
        assert!(!Path::new(&backup_path).exists());
    }

    #[test]
    fn test_db_backup_overwrite() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let backup_path = dir.path().join("backup.db").to_string_lossy().to_string();
        invoke(
            &webview,
            "db_backup",
            serde_json::json!({ "backupPath": backup_path }),
        )
        .expect("first backup should succeed");
        assert!(Path::new(&backup_path).exists());

        invoke(
            &webview,
            "db_backup",
            serde_json::json!({ "backupPath": backup_path }),
        )
        .expect("second backup (overwrite) should succeed");
        assert!(Path::new(&backup_path).exists());
        Database::open(&std::path::PathBuf::from(&backup_path), "secret")
            .expect("overwritten backup must be valid");
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::TempDir;

    /// A plain (unencrypted) database with one row, standing in for the real one.
    fn seeded_db(dir: &TempDir) -> (PathBuf, Connection) {
        let path = dir.path().join("app.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings (key, value) VALUES ('theme', 'dark');",
        )
        .unwrap();
        (path, conn)
    }

    fn leftovers(dir: &TempDir) -> Vec<String> {
        std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|name| name.contains(".tmp-backup"))
            .collect()
    }

    #[test]
    fn backup_writes_a_readable_copy() {
        let dir = TempDir::new().unwrap();
        let (db_path, conn) = seeded_db(&dir);
        let dest = dir.path().join("backup.db");

        backup_database(&conn, &db_path, &dest).unwrap();

        let copy = Connection::open(&dest).unwrap();
        let value: String = copy
            .query_row("SELECT value FROM settings WHERE key = 'theme'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(value, "dark");
        assert!(leftovers(&dir).is_empty(), "temp file must not survive");
    }

    #[test]
    fn backup_refuses_to_overwrite_the_live_database() {
        let dir = TempDir::new().unwrap();
        let (db_path, conn) = seeded_db(&dir);

        let err = backup_database(&conn, &db_path, &db_path).unwrap_err();

        assert!(
            err.contains("数据库文件") || err.contains("database"),
            "unexpected message: {err}"
        );
        assert!(db_path.exists(), "live database must be untouched");
        // The connection is still usable: no data was lost.
        let count: i64 = conn
            .query_row("SELECT count(*) FROM settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn backup_rejects_missing_directory_without_leaving_files() {
        let dir = TempDir::new().unwrap();
        let (db_path, conn) = seeded_db(&dir);
        let dest = dir.path().join("nope").join("backup.db");

        let err = backup_database(&conn, &db_path, &dest).unwrap_err();

        assert!(!err.is_empty());
        assert!(!dest.exists());
        assert!(leftovers(&dir).is_empty());
    }
}
