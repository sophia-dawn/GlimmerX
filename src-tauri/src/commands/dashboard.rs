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

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::accounts::{create_account_with_path, CreateAccountFullInput};
    use crate::db::categories::{create_category, CategoryType};
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
                dashboard_summary,
                dashboard_monthly_chart,
                dashboard_category_breakdown,
                dashboard_top_expenses,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        (app, webview)
    }

    fn mock_app_locked() -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(AppState {
                database: Mutex::new(None),
                recent_dbs: Mutex::new(RecentDbs::empty()),
            })
            .invoke_handler(tauri::generate_handler![
                dashboard_summary,
                dashboard_monthly_chart,
                dashboard_category_breakdown,
                dashboard_top_expenses,
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

    fn seed_data(db: &Database) -> (String, String) {
        let conn = db.get_conn().unwrap();
        let cash = create_account_with_path(
            &conn,
            "Assets/Cash",
            "CNY",
            Some(&CreateAccountFullInput {
                description: None,
                account_number: None,
                iban: None,
                is_active: true,
                include_net_worth: true,
            }),
        )
        .unwrap();
        let salary_account = create_account_with_path(&conn, "Income/Salary", "CNY", None).unwrap();
        let food_account = create_account_with_path(&conn, "Expenses/Food", "CNY", None).unwrap();
        let salary_category =
            create_category(&conn, "Salary", &CategoryType::Income, None).unwrap();
        let food_category = create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();

        create_transaction(
            &conn,
            "2024-01-05",
            "Salary Jan",
            Some(&salary_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: 100000,
                },
                PostingInput {
                    account_id: salary_account.clone(),
                    amount: -100000,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            "2024-01-10",
            "Lunch",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -3500,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 3500,
                },
            ],
        )
        .unwrap();
        (cash, food_account)
    }

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    #[test]
    fn test_dashboard_commands_via_ipc() {
        let (_dir, db) = test_db();
        seed_data(&db);
        let (_app, webview) = mock_app_with_db(db);

        let summary = invoke(
            &webview,
            "dashboard_summary",
            serde_json::json!({
                "fromDate": "2024-01-01",
                "toDate": "2024-01-31"
            }),
        )
        .expect("dashboard_summary should succeed");
        assert_eq!(summary["month_income"], serde_json::json!(100000));
        assert_eq!(summary["month_expense"], serde_json::json!(3500));
        assert_eq!(summary["net_worth"], serde_json::json!(96500));

        let chart = invoke(
            &webview,
            "dashboard_monthly_chart",
            serde_json::json!({ "year": 2024, "month": 1 }),
        )
        .expect("dashboard_monthly_chart should succeed");
        assert_eq!(chart["year"], serde_json::json!(2024));
        assert_eq!(chart["month_total_income"], serde_json::json!(100000));

        let breakdown = invoke(
            &webview,
            "dashboard_category_breakdown",
            serde_json::json!({ "year": 2024, "month": 1, "categoryType": "expense" }),
        )
        .expect("dashboard_category_breakdown should succeed");
        assert_eq!(breakdown["total_amount"], serde_json::json!(3500));
        assert_eq!(breakdown["categories"].as_array().unwrap().len(), 1);

        let top = invoke(
            &webview,
            "dashboard_top_expenses",
            serde_json::json!({ "year": 2024, "month": 1, "limit": 5 }),
        )
        .expect("dashboard_top_expenses should succeed");
        assert_eq!(top["expenses"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_dashboard_category_breakdown_invalid_type() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let err = invoke(
            &webview,
            "dashboard_category_breakdown",
            serde_json::json!({ "year": 2024, "month": 1, "categoryType": "invalid" }),
        )
        .expect_err("invalid category type must fail");
        assert!(err.as_str().unwrap().contains("Invalid category_type"));
    }

    #[test]
    fn test_dashboard_commands_locked_db() {
        let (_app, webview) = mock_app_locked();

        let err = invoke(&webview, "dashboard_summary", serde_json::json!({}))
            .expect_err("locked db must fail");
        assert!(err.as_str().unwrap().contains("not unlocked"));
    }
}
