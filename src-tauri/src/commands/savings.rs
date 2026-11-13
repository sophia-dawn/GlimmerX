//! Savings-goal commands.

use tauri::State;

use crate::db::savings::SavingsGoalRecord;
use crate::utils::money::parse_amount_to_cents;
use crate::AppState;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavingsGoalInput {
    pub name: String,
    /// Decimal amount string, e.g. "20000.00".
    pub target_amount: String,
    pub target_date: Option<String>,
    /// Optional decimal amount string per month.
    pub monthly_plan: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSavingsGoalInput {
    pub name: Option<String>,
    pub target_amount: Option<String>,
    /// `Some(None)` clears the date; omit to keep it.
    #[serde(default, with = "serde_with::rust::double_option")]
    pub target_date: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub monthly_plan: Option<Option<String>>,
}

fn optional_cents(raw: Option<String>) -> Result<Option<i64>, String> {
    match raw {
        Some(value) if !value.trim().is_empty() => parse_amount_to_cents(&value)
            .map(Some)
            .map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

fn with_conn<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, crate::db::AppError>,
) -> Result<T, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or(crate::errors::message("errors.databaseLocked"))?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn savings_list(state: State<'_, AppState>) -> Result<Vec<SavingsGoalRecord>, String> {
    with_conn(&state, crate::db::savings::list_goals)
}

#[tauri::command]
pub async fn savings_create(
    input: CreateSavingsGoalInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let target = parse_amount_to_cents(&input.target_amount).map_err(|e| e.to_string())?;
    let plan = optional_cents(input.monthly_plan)?;
    with_conn(&state, |conn| {
        crate::db::savings::create_goal(
            conn,
            &input.name,
            target,
            input.target_date.as_deref(),
            plan,
        )
    })
}

#[tauri::command]
pub async fn savings_update(
    id: String,
    input: UpdateSavingsGoalInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let target = match input.target_amount {
        Some(value) => Some(parse_amount_to_cents(&value).map_err(|e| e.to_string())?),
        None => None,
    };
    let monthly_plan = match input.monthly_plan {
        Some(Some(value)) => Some(optional_cents(Some(value))?),
        Some(None) => Some(None),
        None => None,
    };
    let target_date = input.target_date;
    let target_date = target_date.as_ref().map(|value| value.as_deref());

    with_conn(&state, |conn| {
        crate::db::savings::update_goal(
            conn,
            &id,
            input.name.as_deref(),
            target,
            target_date,
            monthly_plan,
        )
    })
}

#[tauri::command]
pub async fn savings_set_status(
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_conn(&state, |conn| {
        crate::db::savings::set_status(conn, &id, &status)
    })
}

#[tauri::command]
pub async fn savings_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_conn(&state, |conn| crate::db::savings::delete_goal(conn, &id))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

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
                savings_list,
                savings_create,
                savings_update,
                savings_set_status,
                savings_delete,
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
    fn test_savings_lifecycle_via_ipc() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let id = invoke(
            &webview,
            "savings_create",
            serde_json::json!({
                "input": {
                    "name": "日本旅行",
                    "targetAmount": "10000.00",
                    "targetDate": "2025-06-01",
                    "monthlyPlan": "500.00",
                }
            }),
        )
        .expect("create should succeed");
        let id = id.as_str().unwrap().to_string();

        let goals: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "savings_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0]["name"], serde_json::json!("日本旅行"));
        assert_eq!(goals[0]["targetAmount"], serde_json::json!(1_000_000));
        assert_eq!(goals[0]["monthlyPlan"], serde_json::json!(50_000));
        assert_eq!(goals[0]["savedAmount"], serde_json::json!(0));
        assert_eq!(goals[0]["remaining"], serde_json::json!(1_000_000));
        assert_eq!(goals[0]["status"], serde_json::json!("active"));

        // Target can be raised; an explicit `null` plan clears it.
        invoke(
            &webview,
            "savings_update",
            serde_json::json!({
                "id": id,
                "input": { "targetAmount": "20000.00", "monthlyPlan": null }
            }),
        )
        .expect("update should succeed");

        let goals: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "savings_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert_eq!(goals[0]["targetAmount"], serde_json::json!(2_000_000));
        assert_eq!(goals[0]["monthlyPlan"], serde_json::json!(null));

        invoke(
            &webview,
            "savings_set_status",
            serde_json::json!({ "id": id, "status": "archived" }),
        )
        .expect("archiving should succeed");
        let goals: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "savings_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert_eq!(goals[0]["status"], serde_json::json!("archived"));

        invoke(&webview, "savings_delete", serde_json::json!({ "id": id }))
            .expect("delete should succeed");
        let goals: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "savings_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(goals.is_empty());
    }

    #[test]
    fn test_savings_rejects_bad_input() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let err = invoke(
            &webview,
            "savings_create",
            serde_json::json!({ "input": { "name": "旅行", "targetAmount": "abc" } }),
        )
        .expect_err("unparseable amount must fail");
        assert!(!err.as_str().unwrap().is_empty());

        let err = invoke(
            &webview,
            "savings_create",
            serde_json::json!({ "input": { "name": "旅行", "targetAmount": "0" } }),
        )
        .expect_err("zero target must fail");
        assert!(err.as_str().unwrap().contains("目标金额"));

        invoke(
            &webview,
            "savings_create",
            serde_json::json!({ "input": { "name": "旅行", "targetAmount": "100.00" } }),
        )
        .expect("first goal should succeed");

        let err = invoke(
            &webview,
            "savings_create",
            serde_json::json!({ "input": { "name": "旅行", "targetAmount": "100.00" } }),
        )
        .expect_err("duplicate name must fail");
        assert!(err.as_str().unwrap().contains("已存在"));

        let err = invoke(
            &webview,
            "savings_set_status",
            serde_json::json!({ "id": "missing", "status": "archived" }),
        )
        .expect_err("unknown goal must fail");
        assert!(!err.as_str().unwrap().is_empty());
    }
}
