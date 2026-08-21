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
    /// Decimal amount string (e.g. "500.00"); converted to cents on the backend.
    pub amount: String,
    pub period: BudgetPeriod,
    pub rollover: Option<bool>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBudgetInput {
    pub amount: Option<String>,
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
    let amount =
        crate::db::transactions::parse_amount_to_cents(&input.amount).map_err(|e| e.to_string())?;
    let id = crate::db::budgets::create_budget(
        &tx,
        &input.category_id,
        amount,
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
    let amount = input
        .amount
        .as_deref()
        .map(crate::db::transactions::parse_amount_to_cents)
        .transpose()
        .map_err(|e| e.to_string())?;
    crate::db::budgets::update_budget(
        &tx,
        &crate::db::budgets::UpdateBudgetParams {
            id: &id,
            amount,
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

#[cfg(test)]
mod ipc_tests {
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
                crate::commands::categories::category_create,
                budget_list,
                budget_list_statuses,
                budget_create,
                budget_update,
                budget_delete,
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
    fn test_budget_crud_via_ipc() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        // create an expense category to attach the budget to
        let category = invoke(
            &webview,
            "category_create",
            serde_json::json!({
                "input": { "name": "餐饮", "type": "expense" }
            }),
        )
        .expect("create category should succeed");
        let category_id = category["id"].as_str().unwrap().to_string();

        // create budget with decimal string amount (backend converts to cents)
        let created = invoke(
            &webview,
            "budget_create",
            serde_json::json!({
                "input": {
                    "categoryId": category_id,
                    "amount": "500.00",
                    "period": "monthly",
                    "rollover": false
                }
            }),
        )
        .expect("create budget should succeed");
        let budget_id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["amount"], serde_json::json!(50000));
        assert_eq!(created["period"], serde_json::json!("monthly"));

        // list
        let list: Vec<serde_json::Value> =
            serde_json::from_value(invoke(&webview, "budget_list", serde_json::json!({})).unwrap())
                .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["amount"], serde_json::json!(50000));

        // update amount (decimal string)
        let updated = invoke(
            &webview,
            "budget_update",
            serde_json::json!({
                "id": budget_id,
                "input": { "amount": "600.00", "period": "monthly" }
            }),
        )
        .expect("update budget should succeed");
        assert_eq!(updated["amount"], serde_json::json!(60000));

        // duplicate budget for the same category fails
        let err = invoke(
            &webview,
            "budget_create",
            serde_json::json!({
                "input": {
                    "categoryId": category_id,
                    "amount": "100.00",
                    "period": "monthly"
                }
            }),
        )
        .expect_err("duplicate budget must fail");
        assert!(!err.as_str().unwrap().is_empty());

        // statuses
        let statuses: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "budget_list_statuses",
                serde_json::json!({ "year": 2026, "month": 8 }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(statuses.len(), 1);

        // delete
        invoke(
            &webview,
            "budget_delete",
            serde_json::json!({ "id": budget_id }),
        )
        .unwrap();
        let list: Vec<serde_json::Value> =
            serde_json::from_value(invoke(&webview, "budget_list", serde_json::json!({})).unwrap())
                .unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_budget_create_invalid_amount_fails() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let category = invoke(
            &webview,
            "category_create",
            serde_json::json!({
                "input": { "name": "交通", "type": "expense" }
            }),
        )
        .unwrap();
        let category_id = category["id"].as_str().unwrap().to_string();

        let err = invoke(
            &webview,
            "budget_create",
            serde_json::json!({
                "input": {
                    "categoryId": category_id,
                    "amount": "not-a-number",
                    "period": "monthly"
                }
            }),
        )
        .expect_err("invalid amount must fail");
        assert!(err
            .as_str()
            .unwrap()
            .contains("errors.transaction.invalidAmount"));

        // zero amount fails at the DB layer
        let err = invoke(
            &webview,
            "budget_create",
            serde_json::json!({
                "input": {
                    "categoryId": category_id,
                    "amount": "0.00",
                    "period": "monthly"
                }
            }),
        )
        .expect_err("zero amount must fail");
        assert!(err
            .as_str()
            .unwrap()
            .contains("errors.budgetAmountPositive"));
    }

    #[test]
    fn test_budget_update_invalid_amount_fails() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let category = invoke(
            &webview,
            "category_create",
            serde_json::json!({
                "input": { "name": "Utilities", "type": "expense" }
            }),
        )
        .unwrap();
        let category_id = category["id"].as_str().unwrap().to_string();

        let created = invoke(
            &webview,
            "budget_create",
            serde_json::json!({
                "input": {
                    "categoryId": category_id,
                    "amount": "200.00",
                    "period": "monthly"
                }
            }),
        )
        .unwrap();
        let budget_id = created["id"].as_str().unwrap().to_string();

        let err = invoke(
            &webview,
            "budget_update",
            serde_json::json!({
                "id": budget_id,
                "input": { "amount": "not-a-number" }
            }),
        )
        .expect_err("invalid amount on update must fail");
        assert!(err
            .as_str()
            .unwrap()
            .contains("errors.transaction.invalidAmount"));
    }

    #[test]
    fn test_budget_update_zero_amount_fails() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let category = invoke(
            &webview,
            "category_create",
            serde_json::json!({
                "input": { "name": "Health", "type": "expense" }
            }),
        )
        .unwrap();
        let category_id = category["id"].as_str().unwrap().to_string();

        let created = invoke(
            &webview,
            "budget_create",
            serde_json::json!({
                "input": {
                    "categoryId": category_id,
                    "amount": "100.00",
                    "period": "monthly"
                }
            }),
        )
        .unwrap();
        let budget_id = created["id"].as_str().unwrap().to_string();

        let err = invoke(
            &webview,
            "budget_update",
            serde_json::json!({
                "id": budget_id,
                "input": { "amount": "0.00" }
            }),
        )
        .expect_err("zero amount on update must fail");
        assert!(err
            .as_str()
            .unwrap()
            .contains("errors.budgetAmountPositive"));
    }

    #[test]
    fn test_budget_delete_nonexistent() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        let (_app, webview) = mock_app_with_db(db);

        let err = invoke(
            &webview,
            "budget_delete",
            serde_json::json!({ "id": "nonexistent-id" }),
        )
        .expect_err("deleting nonexistent budget should fail");
        assert!(err.as_str().unwrap().contains("budgetNotFound"));
    }
}
