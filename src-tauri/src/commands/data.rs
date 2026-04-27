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
    std::fs::copy(&db.path, backup_path).map_err(|e| format!("errors.backupFailed: {}", e))?;

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
