//! Data management commands: backup, export, and import.
//!
//! Provides Tauri commands for database backup, transaction export, and import.

use std::path::Path;
use tauri::State;

use crate::db::import::{import_csv, ImportOptions, ImportResult};
use crate::db::{export_beancount, export_csv, ExportResult};
use crate::AppState;

/// Backup the current database to a specified path.
///
/// Performs a WAL checkpoint first, then copies the database file.
/// The backup retains the original encryption password.
#[tauri::command]
pub async fn db_backup(backup_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "errors.databaseLocked".to_string())?;

    let conn = db.get_conn().map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Checkpoint failed: {}", e))?;

    let backup_path = Path::new(&backup_path);
    // Copy to a temporary file first, then rename into place so an interrupted
    // backup never leaves a truncated file at the destination path.
    let tmp_path = backup_path.with_extension("tmp-backup");
    std::fs::copy(&db.path, &tmp_path).map_err(|e| format!("errors.backupFailed: {}", e))?;
    if backup_path.exists() {
        std::fs::remove_file(backup_path).map_err(|e| format!("errors.backupFailed: {}", e))?;
    }
    std::fs::rename(&tmp_path, backup_path).map_err(|e| format!("errors.backupFailed: {}", e))?;

    Ok(())
}

/// Export transactions to CSV format.
///
/// Optionally filtered by date range (YYYY-MM-DD format).
#[tauri::command]
pub async fn export_transactions_csv(
    output_path: String,
    start_date: Option<String>,
    end_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "errors.databaseLocked".to_string())?;

    let conn = db.get_conn().map_err(|e| e.to_string())?;
    let output_path = Path::new(&output_path);

    export_csv(
        &conn,
        output_path,
        start_date.as_deref(),
        end_date.as_deref(),
    )
}

/// Export transactions to Beancount format.
///
/// Optionally filtered by date range (YYYY-MM-DD format).
#[tauri::command]
pub async fn export_transactions_beancount(
    output_path: String,
    start_date: Option<String>,
    end_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "errors.databaseLocked".to_string())?;

    let conn = db.get_conn().map_err(|e| e.to_string())?;
    let output_path = Path::new(&output_path);

    export_beancount(
        &conn,
        output_path,
        start_date.as_deref(),
        end_date.as_deref(),
    )
}

#[tauri::command]
pub async fn import_transactions_csv(
    input_path: String,
    create_missing_accounts: bool,
    skip_duplicates: bool,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "errors.databaseLocked".to_string())?;

    let mut conn = db.get_conn().map_err(|e| e.to_string())?;
    let input_path = Path::new(&input_path);
    let options = ImportOptions {
        create_missing_accounts,
        skip_duplicates,
    };

    import_csv(&mut conn, input_path, &options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::transactions::{create_transaction, PostingInput};
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
                export_transactions_csv,
                export_transactions_beancount,
                import_transactions_csv,
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

    fn seed_transaction(db: &Database) {
        let conn = db.get_conn().unwrap();
        // Two accounts needed for a balanced transaction
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('acc-1', 'Cash', 'asset', 'CNY', '', 0, '2024-01-01T00:00:00+08:00', '2024-01-01T00:00:00+08:00'),
                    ('acc-2', 'Food', 'expense', 'CNY', '', 0, '2024-01-01T00:00:00+08:00', '2024-01-01T00:00:00+08:00')",
            [],
        )
        .unwrap();
        create_transaction(
            &conn,
            "2024-01-15",
            "Grocery",
            None,
            &[
                PostingInput {
                    account_id: "acc-1".to_string(),
                    amount: -3500,
                },
                PostingInput {
                    account_id: "acc-2".to_string(),
                    amount: 3500,
                },
            ],
        )
        .unwrap();
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
    fn test_db_backup_locked_fails() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let backup_path = dir.path().join("backup.db").to_string_lossy().to_string();
        invoke(
            &webview,
            "db_backup",
            serde_json::json!({ "backupPath": backup_path }),
        )
        .unwrap();
        invoke(&webview, "db_lock", serde_json::json!({})).unwrap_err();
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

    #[test]
    fn test_export_csv_and_beancount() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        seed_transaction(&db);
        let (_app, webview) = mock_app_with_db(db);

        let csv_path = dir.path().join("out.csv").to_string_lossy().to_string();
        let csv_result = invoke(
            &webview,
            "export_transactions_csv",
            serde_json::json!({ "outputPath": csv_path }),
        )
        .expect("csv export should succeed");
        assert_eq!(csv_result["transactionCount"], serde_json::json!(1));
        let csv_content = std::fs::read_to_string(&csv_path).unwrap();
        assert!(csv_content.contains("Grocery"));

        let bc_path = dir.path().join("out.bean").to_string_lossy().to_string();
        invoke(
            &webview,
            "export_transactions_beancount",
            serde_json::json!({ "outputPath": bc_path }),
        )
        .expect("beancount export should succeed");
        let bc_content = std::fs::read_to_string(&bc_path).unwrap();
        assert!(bc_content.contains("Grocery"));
    }

    #[test]
    fn test_import_csv_via_ipc() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        // First export produces a CSV we can import back
        let csv_path = dir.path().join("in.csv").to_string_lossy().to_string();
        // Build a minimal valid import CSV (2 postings per transaction)
        std::fs::write(
            &csv_path,
            "transaction_id,date,description,currency,account,account_type,amount,category,reconciled\n\
             txn-1,2024-02-01,Lunch,CNY,Cash,asset,-2500,,false\n\
             txn-1,2024-02-01,Lunch,CNY,Food,expense,2500,,false\n",
        )
        .unwrap();

        let result = invoke(
            &webview,
            "import_transactions_csv",
            serde_json::json!({
                "inputPath": csv_path,
                "createMissingAccounts": true,
                "skipDuplicates": true
            }),
        )
        .expect("import should succeed");
        assert_eq!(result["importedCount"], serde_json::json!(1));
        assert_eq!(result["errorCount"], serde_json::json!(0));
    }
}
