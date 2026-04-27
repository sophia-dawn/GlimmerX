use tauri::State;

use crate::db::budgets::{BudgetPeriod, BudgetRecord, BudgetStatusRecord};
use crate::AppState;

// ---------------------------------------------------------------------------
// DTO types (serializable for Tauri IPC)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetDto {
    pub id: String,
    pub category_id: String,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatusDto {
    pub id: String,
    pub category_id: String,
    pub category_name: String,
    pub category_icon: Option<String>,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: bool,
    pub spent: i64,
    pub remaining: i64,
    pub over_budget: bool,
    pub rollover_amount: i64,
    pub available: i64,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBudgetInput {
    pub category_id: String,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: Option<bool>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBudgetInput {
    pub amount: Option<i64>,
    pub period: Option<BudgetPeriod>,
    pub rollover: Option<bool>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn record_to_dto(record: BudgetRecord) -> BudgetDto {
    BudgetDto {
        id: record.id,
        category_id: record.category_id,
        amount: record.amount,
        period: record.period,
        rollover: record.rollover,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn status_to_dto(status: BudgetStatusRecord) -> BudgetStatusDto {
    BudgetStatusDto {
        id: status.id,
        category_id: status.category_id,
        category_name: status.category_name,
        category_icon: status.category_icon,
        amount: status.amount,
        period: status.period,
        rollover: status.rollover,
        spent: status.spent,
        remaining: status.remaining,
        over_budget: status.over_budget,
        rollover_amount: status.rollover_amount,
        available: status.available,
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn budget_list(state: State<'_, AppState>) -> Result<Vec<BudgetDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let budgets = crate::db::budgets::list_budgets(&conn).map_err(|e| e.to_string())?;

    Ok(budgets.into_iter().map(record_to_dto).collect())
}

#[tauri::command]
pub async fn budget_list_statuses(
    year: i32,
    month: i32,
    state: State<'_, AppState>,
) -> Result<Vec<BudgetStatusDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let statuses =
        crate::db::budgets::list_budget_statuses(&conn, year, month).map_err(|e| e.to_string())?;

    Ok(statuses.into_iter().map(status_to_dto).collect())
}

#[tauri::command]
pub async fn budget_create(
    input: CreateBudgetInput,
    state: State<'_, AppState>,
) -> Result<BudgetDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let id = crate::db::budgets::create_budget(
        &tx,
        &input.category_id,
        input.amount,
        &input.period,
        input.rollover.unwrap_or(false),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    let budget = crate::db::budgets::find_budget(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "errors.budgetNotFound".to_string())?;

    Ok(record_to_dto(budget))
}

#[tauri::command]
pub async fn budget_update(
    id: String,
    input: UpdateBudgetInput,
    state: State<'_, AppState>,
) -> Result<BudgetDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    crate::db::budgets::update_budget(
        &tx,
        &crate::db::budgets::UpdateBudgetParams {
            id: &id,
            amount: input.amount,
            period: input.period,
            rollover: input.rollover,
        },
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    let budget = crate::db::budgets::find_budget(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "errors.budgetNotFound".to_string())?;

    Ok(record_to_dto(budget))
}

#[tauri::command]
pub async fn budget_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    crate::db::budgets::delete_budget(&tx, &id).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}
