use std::sync::Mutex;

mod commands;
mod constants;
mod db;
mod utils;

use db::{Database, RecentDbs};
use tauri::{Manager, WindowEvent};

/// Shared application state, accessible from Tauri commands.
pub struct AppState {
    pub database: Mutex<Option<Database>>,
    pub recent_dbs: Mutex<RecentDbs>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recent_dbs = RecentDbs::load().unwrap_or_else(|e| {
        eprintln!(
            "Warning: failed to load recent dbs: {}, using empty list",
            e
        );
        RecentDbs::empty()
    });

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
                        if db_state.is_some() {
                            eprintln!("[window_close] database exists, closing");
                            // Drop will handle checkpoint
                            *db_state = None;
                            eprintln!("[window_close] database closed");
                        } else {
                            eprintln!("[window_close] no database to close");
                        }
                    } else {
                        eprintln!("[window_close] failed to lock database state");
                    }
                } else {
                    eprintln!("[window_close] AppState not available");
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
            commands::reports::report_month_comparison,
            commands::reports::report_category_breakdown,
            commands::reports::report_balance_sheet,
            commands::reports::report_trend,
            commands::reports::report_year_summary,
            commands::reports::report_account_transactions,
            commands::reports::report_account_balance_trend,
            commands::reports::report_audit,
            commands::data::db_backup,
            commands::data::export_transactions_csv,
            commands::data::export_transactions_beancount,
            commands::data::import_transactions_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
