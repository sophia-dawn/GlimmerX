use tauri::State;

use crate::db::reports::{
    self, AccountBalanceTrendReportDto, AccountTransactionsReportDto, AuditReportDto,
    BalanceSheetReportDto, CategoryBreakdownReportDto, MonthComparisonReportDto, ReportFilter,
    StandardReportDto, TrendReportDto, YearSummaryReportDto,
};
use crate::AppState;

#[tauri::command]
pub async fn report_standard(
    filter: ReportFilter,
    state: State<'_, AppState>,
) -> Result<StandardReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_standard_report(&conn, &filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_month_comparison(
    month1: String,
    month2: String,
    state: State<'_, AppState>,
) -> Result<MonthComparisonReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_month_comparison(&conn, &month1, &month2).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_category_breakdown(
    filter: ReportFilter,
    income_or_expense: String,
    state: State<'_, AppState>,
) -> Result<CategoryBreakdownReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_category_breakdown_report(&conn, &filter, &income_or_expense)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_balance_sheet(
    snapshot_date: String,
    state: State<'_, AppState>,
) -> Result<BalanceSheetReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_balance_sheet_report(&conn, &snapshot_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_trend(
    filter: ReportFilter,
    state: State<'_, AppState>,
) -> Result<TrendReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_trend_report(&conn, &filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_year_summary(
    year: i32,
    state: State<'_, AppState>,
) -> Result<YearSummaryReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_year_summary_report(&conn, year).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_account_transactions(
    account_id: String,
    filter: ReportFilter,
    page: u32,
    page_size: u32,
    state: State<'_, AppState>,
) -> Result<AccountTransactionsReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_account_transactions_report(&conn, &account_id, &filter, page, page_size)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_account_balance_trend(
    account_id: String,
    filter: ReportFilter,
    state: State<'_, AppState>,
) -> Result<AccountBalanceTrendReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_account_balance_trend_report(&conn, &account_id, &filter)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_audit(state: State<'_, AppState>) -> Result<AuditReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_audit_report(&conn).map_err(|e| e.to_string())
}
