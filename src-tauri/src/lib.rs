use std::sync::Mutex;

mod commands;
mod constants;
mod db;
mod errors;
mod services;
mod utils;

use db::{Database, RecentDbs};
use tauri::{Manager, WindowEvent};

/// Shared application state, accessible from Tauri commands.
pub struct AppState {
    pub database: Mutex<Option<Database>>,
    pub recent_dbs: Mutex<RecentDbs>,
}

/// Portable builds have no installer to bootstrap WebView2. Detect the
/// runtime before the webview is created so users get a readable message
/// instead of a silent white window.
#[cfg(windows)]
fn ensure_webview_runtime() {
    if tauri::webview_version().is_ok() {
        return;
    }
    let message = "未检测到 WebView2 运行时，请先安装后重试。 / WebView2 Runtime not found. Please install it and try again.";
    eprintln!("{message}");
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('未检测到 WebView2 运行时，请先安装后重试。 / WebView2 Runtime not found.', 'GlimmerX', 'OK', 'Error')",
        ])
        .status();
    std::process::exit(1);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    ensure_webview_runtime();

    let recent_dbs = RecentDbs::load().unwrap_or_else(|_| RecentDbs::empty());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                // Attempt to close database before window closes
                if let Some(state) = window.try_state::<AppState>() {
                    if let Ok(mut db_state) = state.database.lock() {
                        // Drop will handle checkpoint
                        *db_state = None;
                    }
                }
            }
        })
        .manage(AppState {
            database: Mutex::new(None),
            recent_dbs: Mutex::new(recent_dbs),
        })
        .invoke_handler(tauri::generate_handler![
            commands::db::db_create,
            commands::db::db_unlock,
            commands::db::db_change_password,
            commands::db::db_check_exists,
            commands::db::db_check_any_exists,
            commands::db::db_list_recent,
            commands::db::db_remove_recent,
            commands::db::db_lock,
            commands::db::db_is_unlocked,
            commands::db::db_ping,
            commands::accounts::account_create,
            commands::accounts::account_list,
            commands::accounts::account_update,
            commands::accounts::account_delete,
            commands::accounts::account_balance,
            commands::accounts::account_balances_batch,
            commands::accounts::account_transfer,
            commands::accounts::account_batch_create,
            commands::accounts::account_transactions,
            commands::accounts::account_meta_get,
            commands::accounts::account_meta_set,
            commands::accounts::account_meta_batch_set,
            commands::accounts::account_meta_schema,
            commands::categories::category_list,
            commands::categories::category_create,
            commands::categories::category_update,
            commands::categories::category_delete,
            commands::categories::category_delete_preview,
            commands::budgets::budget_list,
            commands::budgets::budget_list_statuses,
            commands::budgets::budget_create,
            commands::budgets::budget_update,
            commands::budgets::budget_delete,
            commands::transactions::transaction_create,
            commands::transactions::transaction_list,
            commands::transactions::transaction_get,
            commands::transactions::transaction_list_paginated,
            commands::transactions::quick_add_transaction,
            commands::transactions::transaction_detail,
            commands::transactions::transaction_update,
            commands::transactions::transaction_delete_preview,
            commands::transactions::transaction_delete,
            commands::dashboard::dashboard_summary,
            commands::dashboard::dashboard_monthly_chart,
            commands::dashboard::dashboard_category_breakdown,
            commands::dashboard::dashboard_top_expenses,
            commands::reports::report_standard,
            commands::reports::report_category_breakdown,
            commands::reports::report_balance_sheet,
            commands::reports::report_trend,
            commands::reports::report_account_transactions,
            commands::reports::report_account_balance_trend,
            commands::reports::report_audit,
            commands::data::db_backup,
            commands::reconcile::reconcile_preview,
            commands::reconcile::reconcile_auto_match,
            commands::reconcile::reconcile_totals,
            commands::reconcile::reconcile_create,
            commands::reconcile::reconcile_list,
            commands::reconcile::reconcile_undo,
            commands::savings::savings_list,
            commands::savings::savings_create,
            commands::savings::savings_update,
            commands::savings::savings_set_status,
            commands::savings::savings_delete,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::set_ui_language,
            commands::ai::ai_parse_transaction,
            commands::ai::ai_test_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
