use tauri::State;

use crate::db::categories::{CategoryRecord, CategoryType, IconUpdateAction};
use crate::AppState;

// ---------------------------------------------------------------------------
// DTO types (serializable for Tauri IPC)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub icon: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(serde::Serialize)]
pub struct DeletePreviewDto {
    #[serde(rename = "budgetCount")]
    pub budget_count: i64,
    #[serde(rename = "transactionCount")]
    pub transaction_count: i64,
    #[serde(rename = "canDelete")]
    pub can_delete: bool,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
pub struct CreateCategoryInput {
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub icon: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UpdateCategoryInput {
    pub name: Option<String>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub icon: Option<Option<String>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn record_to_dto(record: CategoryRecord) -> CategoryDto {
    CategoryDto {
        id: record.id,
        name: record.name,
        category_type: record.category_type,
        icon: record.icon,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn category_list(
    category_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let categories = match category_type {
        Some(t) => {
            let ct = match t.as_str() {
                "income" => CategoryType::Income,
                "expense" => CategoryType::Expense,
                _ => return Err("Invalid category type".to_string()),
            };
            crate::db::categories::list_by_type(&conn, &ct)
        }
        None => crate::db::categories::list_categories(&conn),
    }
    .map_err(|e| e.to_string())?;

    Ok(categories.into_iter().map(record_to_dto).collect())
}

#[tauri::command]
pub async fn category_create(
    input: CreateCategoryInput,
    state: State<'_, AppState>,
) -> Result<CategoryDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let id = crate::db::categories::create_category(
        &tx,
        &input.name,
        &input.category_type,
        input.icon.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    let category = crate::db::categories::find_category(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "errors.categoryNotFound".to_string())?;

    Ok(record_to_dto(category))
}

#[tauri::command]
pub async fn category_update(
    id: String,
    input: UpdateCategoryInput,
    state: State<'_, AppState>,
) -> Result<CategoryDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let icon_action = match input.icon {
        Some(Some(v)) => IconUpdateAction::Set(v),
        Some(None) => IconUpdateAction::Clear,
        None => IconUpdateAction::NoChange,
    };
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    crate::db::categories::update_category(
        &tx,
        &crate::db::categories::UpdateCategoryParams {
            id: &id,
            name: input.name.as_deref(),
            icon: icon_action,
        },
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    let category = crate::db::categories::find_category(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "errors.categoryNotFound".to_string())?;
    Ok(record_to_dto(category))
}

#[tauri::command]
pub async fn category_delete_preview(
    id: String,
    state: State<'_, AppState>,
) -> Result<DeletePreviewDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let preview = crate::db::categories::preview_delete(&conn, &id).map_err(|e| e.to_string())?;

    Ok(DeletePreviewDto {
        budget_count: preview.budget_count,
        transaction_count: preview.transaction_count,
        can_delete: preview.can_delete,
    })
}

#[tauri::command]
pub async fn category_delete(
    id: String,
    cascade: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    crate::db::categories::delete_category(&tx, &id, cascade).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_to_dto_conversion() {
        let record = crate::db::categories::CategoryRecord {
            id: "test-id".to_string(),
            name: "Test Category".to_string(),
            category_type: crate::db::categories::CategoryType::Expense,
            icon: Some("🍔".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let dto = record_to_dto(record);

        assert_eq!(dto.id, "test-id");
        assert_eq!(dto.name, "Test Category");
        assert_eq!(
            dto.category_type,
            crate::db::categories::CategoryType::Expense
        );
        assert_eq!(dto.icon, Some("🍔".to_string()));
        assert_eq!(dto.created_at, "2024-01-01T00:00:00Z");
        assert_eq!(dto.updated_at, "2024-01-01T00:00:00Z");
    }
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
                category_list,
                category_create,
                category_update,
                category_delete_preview,
                category_delete,
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

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    #[test]
    fn test_category_crud_via_ipc() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        // create
        let created = invoke(
            &webview,
            "category_create",
            serde_json::json!({
                "input": { "name": "餐饮", "type": "expense", "icon": "🍔" }
            }),
        )
        .expect("create should succeed");
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["name"], serde_json::json!("餐饮"));
        assert_eq!(created["type"], serde_json::json!("expense"));

        // duplicate (type, name) fails
        let err = invoke(
            &webview,
            "category_create",
            serde_json::json!({ "input": { "name": "餐饮", "type": "expense" } }),
        )
        .expect_err("duplicate category must fail");
        assert!(
            err.as_str().unwrap().contains("already exists")
                || err.as_str().unwrap().contains("errors.")
        );

        // list filters by type
        let list: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "category_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert_eq!(list.len(), 1);

        // update icon
        let updated = invoke(
            &webview,
            "category_update",
            serde_json::json!({ "id": id, "input": { "icon": "🍜" } }),
        )
        .expect("update should succeed");
        assert_eq!(updated["icon"], serde_json::json!("🍜"));

        // delete preview: no transactions, so can delete
        let preview = invoke(
            &webview,
            "category_delete_preview",
            serde_json::json!({ "id": id }),
        )
        .expect("preview should succeed");
        assert_eq!(preview["canDelete"], serde_json::json!(true));
        assert_eq!(preview["transactionCount"], serde_json::json!(0));

        // delete
        invoke(
            &webview,
            "category_delete",
            serde_json::json!({ "id": id, "cascade": false }),
        )
        .unwrap();
        let list: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "category_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_category_not_found_fails() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let err = invoke(
            &webview,
            "category_update",
            serde_json::json!({ "id": "missing", "input": { "name": "X" } }),
        )
        .expect_err("updating missing category must fail");
        assert!(!err.as_str().unwrap().is_empty());

        let err = invoke(
            &webview,
            "category_delete",
            serde_json::json!({ "id": "missing" }),
        )
        .expect_err("deleting missing category must fail");
        assert!(!err.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_category_list_by_type() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        invoke(
            &webview,
            "category_create",
            serde_json::json!({ "input": { "name": "Salary", "type": "income" } }),
        )
        .unwrap();
        invoke(
            &webview,
            "category_create",
            serde_json::json!({ "input": { "name": "Food", "type": "expense" } }),
        )
        .unwrap();

        let income: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "category_list",
                serde_json::json!({ "categoryType": "income" }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(income.len(), 1);
        assert_eq!(income[0]["name"], serde_json::json!("Salary"));

        let expense: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "category_list",
                serde_json::json!({ "categoryType": "expense" }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(expense.len(), 1);
        assert_eq!(expense[0]["name"], serde_json::json!("Food"));

        let err = invoke(
            &webview,
            "category_list",
            serde_json::json!({ "categoryType": "invalid" }),
        )
        .expect_err("invalid category type must fail");
        assert_eq!(err.as_str().unwrap(), "Invalid category type");
    }

    #[test]
    fn test_category_clear_icon() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = invoke(
            &webview,
            "category_create",
            serde_json::json!({ "input": { "name": "ClearMe", "type": "expense", "icon": "🍔" } }),
        )
        .expect("create should succeed");
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["icon"], serde_json::json!("🍔"));

        let updated = invoke(
            &webview,
            "category_update",
            serde_json::json!({ "id": id, "input": { "icon": null } }),
        )
        .expect("clear icon should succeed");
        assert_eq!(updated["icon"], serde_json::json!(null));
    }
}
