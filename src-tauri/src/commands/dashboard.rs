use tauri::State;

use crate::db::categories::CategoryType;
use crate::db::dashboard;
use crate::utils::time;
use crate::AppState;

#[tauri::command]
pub async fn dashboard_summary(
    from_date: Option<String>,
    to_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<dashboard::DashboardSummary, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let (month_start, month_end) = time::current_month_bounds();
    let from_date = from_date.unwrap_or(month_start);
    let to_date = to_date.unwrap_or(month_end);

    let (year_start, year_end) = time::year_bounds(time::current_year());

    dashboard::get_dashboard_summary(&conn, &from_date, &to_date, &year_start, &year_end)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dashboard_monthly_chart(
    year: Option<i32>,
    month: Option<i32>,
    state: State<'_, AppState>,
) -> Result<dashboard::MonthlyChartData, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let year = year.unwrap_or(time::current_year());
    let month = month.unwrap_or(time::current_month() as i32);

    dashboard::get_monthly_chart(&conn, year, month).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dashboard_category_breakdown(
    year: Option<i32>,
    month: Option<i32>,
    category_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<dashboard::CategoryBreakdownData, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let year = year.unwrap_or(time::current_year());
    let month = month.unwrap_or(time::current_month() as i32);

    let category_type = match category_type.as_deref() {
        Some("income") => CategoryType::Income,
        Some("expense") | None => CategoryType::Expense,
        Some(other) => {
            return Err(format!(
                "Invalid category_type: '{}'. Must be 'income' or 'expense'.",
                other
            ))
        }
    };

    dashboard::get_category_breakdown(&conn, year, month, category_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dashboard_top_expenses(
    year: Option<i32>,
    month: Option<i32>,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<dashboard::TopExpensesData, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let year = year.unwrap_or(time::current_year());
    let month = month.unwrap_or(time::current_month() as i32);
    let limit = limit.unwrap_or(10);

    dashboard::get_top_expenses(&conn, year, month, limit).map_err(|e| e.to_string())
}
