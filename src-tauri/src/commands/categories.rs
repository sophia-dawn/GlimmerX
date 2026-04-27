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
    println!("[category_update] id: {}", id);
    println!("[category_update] input.icon: {:?}", input.icon);

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

    println!("[category_update] icon_action: {:?}", icon_action);

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

    println!("[category_update] result icon: {:?}", category.icon);

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
