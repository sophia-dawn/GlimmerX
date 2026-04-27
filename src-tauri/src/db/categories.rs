use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::AppError;

// ---------------------------------------------------------------------------
// Type definitions
// ---------------------------------------------------------------------------

/// Category type enum for income/expense distinction.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryType {
    Income,
    #[default]
    Expense,
}

/// Action for updating icon field in category.
#[derive(Debug, Clone)]
pub enum IconUpdateAction {
    Set(String),
    Clear,
    NoChange,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Internal DB row representation for a category (flat structure).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryRecord {
    pub id: String,
    pub name: String,
    pub category_type: CategoryType,
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Delete preview structure showing references to category.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeletePreview {
    pub budget_count: i64,
    pub transaction_count: i64,
    pub can_delete: bool,
}

// ---------------------------------------------------------------------------
// Constants and helpers
// ---------------------------------------------------------------------------

/// Common columns for SELECT queries.
const CATEGORY_COLUMNS: &str = "id, name, type, icon, created_at, updated_at";

/// Read a CategoryRecord from a row.
fn row_to_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<CategoryRecord> {
    let category_type_str: String = row.get(2)?;
    let category_type = match category_type_str.as_str() {
        "income" => CategoryType::Income,
        "expense" => CategoryType::Expense,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(CategoryRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        category_type,
        icon: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// List all categories, sorted by type then name.
pub fn list_categories(conn: &Connection) -> Result<Vec<CategoryRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM categories ORDER BY type, name",
        CATEGORY_COLUMNS
    ))?;
    let categories = stmt
        .query_map([], row_to_category)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(categories)
}

/// List categories filtered by type, sorted by name.
pub fn list_by_type(
    conn: &Connection,
    category_type: &CategoryType,
) -> Result<Vec<CategoryRecord>, AppError> {
    let type_str = match category_type {
        CategoryType::Income => "income",
        CategoryType::Expense => "expense",
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM categories WHERE type = ?1 ORDER BY name",
        CATEGORY_COLUMNS
    ))?;
    let categories = stmt
        .query_map([type_str], row_to_category)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(categories)
}

/// Find a single category by ID.
pub fn find_category(conn: &Connection, id: &str) -> Result<Option<CategoryRecord>, AppError> {
    let row = conn
        .query_row(
            &format!("SELECT {} FROM categories WHERE id = ?1", CATEGORY_COLUMNS),
            [id],
            row_to_category,
        )
        .optional()?;
    Ok(row)
}

/// Find a category by type and name (for uniqueness check).
pub fn find_by_type_and_name(
    conn: &Connection,
    category_type: &CategoryType,
    name: &str,
) -> Result<Option<CategoryRecord>, AppError> {
    let type_str = match category_type {
        CategoryType::Income => "income",
        CategoryType::Expense => "expense",
    };
    let row = conn
        .query_row(
            &format!(
                "SELECT {} FROM categories WHERE type = ?1 AND name = ?2",
                CATEGORY_COLUMNS
            ),
            rusqlite::params![type_str, name],
            row_to_category,
        )
        .optional()?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// Create operation
// ---------------------------------------------------------------------------

/// Create a new category. Returns the new category ID.
pub fn create_category(
    conn: &Connection,
    name: &str,
    category_type: &CategoryType,
    icon: Option<&str>,
) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::ValidationError(
            "errors.categoryNameRequired".to_string(),
        ));
    }

    // Enum 已保证类型合法，无需手动校验
    let type_str = match category_type {
        CategoryType::Income => "income",
        CategoryType::Expense => "expense",
    };

    if let Some(_existing) = find_by_type_and_name(conn, category_type, name)? {
        return Err(AppError::ValidationError(
            "errors.categoryNameDuplicate".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();

    conn.execute(
        &format!(
            "INSERT INTO categories ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            CATEGORY_COLUMNS
        ),
        rusqlite::params![&id, name, type_str, icon, &now, &now],
    )?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Update operation
// ---------------------------------------------------------------------------

/// Parameters for updating a category.
pub struct UpdateCategoryParams<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub icon: IconUpdateAction,
}

/// Update category fields.
pub fn update_category(
    conn: &Connection,
    params: &UpdateCategoryParams<'_>,
) -> Result<(), AppError> {
    if let Some(new_name) = params.name {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(AppError::ValidationError(
                "errors.categoryNameRequired".to_string(),
            ));
        }

        let current_type: String = conn.query_row(
            "SELECT type FROM categories WHERE id = ?1",
            [&params.id],
            |row| row.get(0),
        )?;

        let current_category_type = match current_type.as_str() {
            "income" => CategoryType::Income,
            "expense" => CategoryType::Expense,
            _ => {
                return Err(AppError::ValidationError(
                    "Invalid category type".to_string(),
                ))
            }
        };

        if let Some(existing) = find_by_type_and_name(conn, &current_category_type, new_name)? {
            if existing.id != params.id {
                return Err(AppError::ValidationError(
                    "errors.categoryNameDuplicate".to_string(),
                ));
            }
        }
    }

    let now = crate::utils::time::now_rfc3339();
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(val) = params.name {
        sets.push("name = ?".to_string());
        sql_params.push(Box::new(val.to_string()));
    }

    match &params.icon {
        IconUpdateAction::Set(v) => {
            sets.push("icon = ?".to_string());
            sql_params.push(Box::new(v.clone()));
        }
        IconUpdateAction::Clear => {
            sets.push("icon = ?".to_string());
            sql_params.push(Box::new(None::<String>));
        }
        IconUpdateAction::NoChange => {}
    }

    if sets.is_empty() {
        return Ok(());
    }

    sets.push("updated_at = ?".to_string());
    sql_params.push(Box::new(now));
    sql_params.push(Box::new(params.id.to_string()));

    let sql = format!("UPDATE categories SET {} WHERE id = ?", sets.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();
    stmt.execute(rusqlite::params_from_iter(param_refs))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete operations
// ---------------------------------------------------------------------------

/// Preview deletion impact for a category.
pub fn preview_delete(conn: &Connection, id: &str) -> Result<DeletePreview, AppError> {
    let _category = find_category(conn, id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    let budget_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM budgets WHERE category_id = ?1",
        [id],
        |row| row.get(0),
    )?;

    let transaction_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions WHERE category_id = ?1",
        [id],
        |row| row.get(0),
    )?;

    Ok(DeletePreview {
        budget_count,
        transaction_count,
        can_delete: true,
    })
}

/// Delete a category. If cascade is true, deletes budgets and unlinks transactions.
pub fn delete_category(conn: &Connection, id: &str, cascade: bool) -> Result<(), AppError> {
    let _category = find_category(conn, id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    // 检查预算关联
    let budget_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM budgets WHERE category_id = ?1",
        [id],
        |row| row.get(0),
    )?;

    // 存在预算且 cascade=false，阻止删除
    if budget_count > 0 && !cascade {
        return Err(AppError::ValidationError(
            "errors.categoryHasBudgets".to_string(),
        ));
    }

    // cascade=true 时删除预算
    if cascade {
        conn.execute("DELETE FROM budgets WHERE category_id = ?1", [id])?;
    }

    // 解除交易关联（category_id 允许 NULL）
    conn.execute(
        "UPDATE transactions SET category_id = NULL WHERE category_id = ?1",
        [id],
    )?;

    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_conn() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE categories (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                type       TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                icon       TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE UNIQUE INDEX idx_categories_type_name ON categories(type, name);
            CREATE INDEX idx_categories_type ON categories(type);

            CREATE TABLE budgets (
                id          TEXT PRIMARY KEY,
                category_id TEXT NOT NULL REFERENCES categories(id),
                amount      INTEGER NOT NULL,
                period      TEXT NOT NULL,
                rollover    INTEGER NOT NULL DEFAULT 0,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE transactions (
                id          TEXT PRIMARY KEY,
                date        TEXT NOT NULL,
                description TEXT NOT NULL,
                category_id TEXT REFERENCES categories(id),
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn test_create_category() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, Some("🍔")).unwrap();
        assert!(!id.is_empty());

        let cat = find_category(&conn, &id).unwrap().unwrap();
        assert_eq!(cat.name, "餐饮");
        assert_eq!(cat.category_type, CategoryType::Expense);
        assert_eq!(cat.icon, Some("🍔".to_string()));
    }

    #[test]
    fn test_create_duplicate_name_same_type_blocked() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let result = create_category(&conn, "餐饮", &CategoryType::Expense, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_same_name_different_type_allowed() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let result = create_category(&conn, "餐饮", &CategoryType::Income, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_by_type() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "工资", &CategoryType::Income, None).unwrap();
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let income_cats = list_by_type(&conn, &CategoryType::Income).unwrap();
        assert_eq!(income_cats.len(), 1);
        assert_eq!(income_cats[0].name, "工资");

        let expense_cats = list_by_type(&conn, &CategoryType::Expense).unwrap();
        assert_eq!(expense_cats.len(), 1);
        assert_eq!(expense_cats[0].name, "餐饮");
    }

    #[test]
    fn test_update_category() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: Some("外卖"),
                icon: IconUpdateAction::Set("🍔".to_string()),
            },
        )
        .unwrap();

        let cat = find_category(&conn, &id).unwrap().unwrap();
        assert_eq!(cat.name, "外卖");
        assert_eq!(cat.icon, Some("🍔".to_string()));
    }

    #[test]
    fn test_delete_category() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        delete_category(&conn, &id, false).unwrap();
        assert!(find_category(&conn, &id).unwrap().is_none());
    }

    #[test]
    fn test_delete_preview() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let preview = preview_delete(&conn, &id).unwrap();
        assert_eq!(preview.budget_count, 0);
        assert_eq!(preview.transaction_count, 0);
        assert!(preview.can_delete);
    }

    #[test]
    fn test_check_constraint_invalid_type() {
        let (_dir, conn) = test_conn();
        let now = crate::utils::time::now_rfc3339();
        let result = conn.execute(
            "INSERT INTO categories (id, name, type, created_at, updated_at) VALUES ('1', 'Test', 'invalid', ?1, ?1)",
            [&now],
        );
        assert!(
            result.is_err(),
            "Invalid type should violate CHECK constraint"
        );
    }

    #[test]
    fn test_delete_with_cascade() {
        let (_dir, conn) = test_conn();
        let cat_id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO budgets (id, category_id, amount, period, rollover, created_at, updated_at) VALUES (?1, ?2, 10000, 'monthly', 0, ?3, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), &cat_id, &now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at) VALUES (?1, '2024-01-01', 'Test', ?2, ?3, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), &cat_id, &now],
        )
        .unwrap();

        let preview = preview_delete(&conn, &cat_id).unwrap();
        assert_eq!(preview.budget_count, 1);
        assert_eq!(preview.transaction_count, 1);

        delete_category(&conn, &cat_id, true).unwrap();
        assert!(find_category(&conn, &cat_id).unwrap().is_none());

        let budget_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM budgets", [], |row| row.get(0))
            .unwrap();
        assert_eq!(budget_count, 0);

        let tx_null_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE category_id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tx_null_count, 1);
    }

    #[test]
    fn test_list_categories_sorted() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "Transport", &CategoryType::Expense, None).unwrap();
        create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();
        create_category(&conn, "Salary", &CategoryType::Income, None).unwrap();
        create_category(&conn, "Bonus", &CategoryType::Income, None).unwrap();

        let cats = list_categories(&conn).unwrap();
        assert_eq!(cats.len(), 4);

        assert_eq!(cats[0].category_type, CategoryType::Expense);
        assert_eq!(cats[0].name, "Food");
        assert_eq!(cats[1].category_type, CategoryType::Expense);
        assert_eq!(cats[1].name, "Transport");
        assert_eq!(cats[2].category_type, CategoryType::Income);
        assert_eq!(cats[2].name, "Bonus");
        assert_eq!(cats[3].category_type, CategoryType::Income);
        assert_eq!(cats[3].name, "Salary");
    }

    #[test]
    fn test_create_empty_name_blocked() {
        let (_dir, conn) = test_conn();
        let result = create_category(&conn, "", &CategoryType::Expense, None);
        assert!(result.is_err());

        let result = create_category(&conn, "   ", &CategoryType::Expense, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_empty_name_blocked() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let result = update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: Some(""),
                icon: IconUpdateAction::NoChange,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_duplicate_name_blocked() {
        let (_dir, conn) = test_conn();
        let _id1 = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let id2 = create_category(&conn, "交通", &CategoryType::Expense, None).unwrap();

        let result = update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id2,
                name: Some("餐饮"),
                icon: IconUpdateAction::NoChange,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_same_name_allowed() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let result = update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: Some("餐饮"),
                icon: IconUpdateAction::Set("🍔".to_string()),
            },
        );
        assert!(result.is_ok());

        let cat = find_category(&conn, &id).unwrap().unwrap();
        assert_eq!(cat.icon, Some("🍔".to_string()));
    }

    #[test]
    fn test_update_clear_icon() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, Some("🍔")).unwrap();

        update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: None,
                icon: IconUpdateAction::Clear,
            },
        )
        .unwrap();

        let cat = find_category(&conn, &id).unwrap().unwrap();
        assert!(cat.icon.is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let (_dir, conn) = test_conn();
        let result = delete_category(&conn, "nonexistent-id", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_preview_delete_nonexistent() {
        let (_dir, conn) = test_conn();
        let result = preview_delete(&conn, "nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_updated_at_on_create() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let cat = find_category(&conn, &id).unwrap().unwrap();
        assert!(!cat.updated_at.is_empty());
        assert!(!cat.created_at.is_empty());
    }

    #[test]
    fn test_updated_at_updated_on_modify() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let cat_before = find_category(&conn, &id).unwrap().unwrap();
        let updated_at_before = cat_before.updated_at.clone();

        std::thread::sleep(std::time::Duration::from_millis(10));

        update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: Some("外卖"),
                icon: IconUpdateAction::NoChange,
            },
        )
        .unwrap();

        let cat_after = find_category(&conn, &id).unwrap().unwrap();
        assert_ne!(cat_after.updated_at, updated_at_before);
    }

    #[test]
    fn test_delete_with_budgets_blocked_without_cascade() {
        let (_dir, conn) = test_conn();
        let cat_id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO budgets (id, category_id, amount, period, rollover, created_at, updated_at) VALUES (?1, ?2, 10000, 'monthly', 0, ?3, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), &cat_id, &now],
        )
        .unwrap();

        // cascade=false + 存在预算 → 应返回错误
        let result = delete_category(&conn, &cat_id, false);
        assert!(result.is_err());

        // 错误消息应为 errors.categoryHasBudgets
        if let Err(AppError::ValidationError(msg)) = result {
            assert_eq!(msg, "errors.categoryHasBudgets");
        } else {
            panic!("Expected ValidationError with categoryHasBudgets");
        }

        // 分类应仍然存在
        assert!(find_category(&conn, &cat_id).unwrap().is_some());
    }

    #[test]
    fn test_delete_with_budgets_cascade_deletes_budgets() {
        let (_dir, conn) = test_conn();
        let cat_id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO budgets (id, category_id, amount, period, rollover, created_at, updated_at) VALUES (?1, ?2, 10000, 'monthly', 0, ?3, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), &cat_id, &now],
        )
        .unwrap();

        // cascade=true + 存在预算 → 应删除预算和分类
        delete_category(&conn, &cat_id, true).unwrap();
        assert!(find_category(&conn, &cat_id).unwrap().is_none());

        let budget_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM budgets", [], |row| row.get(0))
            .unwrap();
        assert_eq!(budget_count, 0);
    }

    #[test]
    fn test_delete_with_transactions_nulls_category_id() {
        let (_dir, conn) = test_conn();
        let cat_id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at) VALUES (?1, '2024-01-01', 'Test', ?2, ?3, ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), &cat_id, &now],
        )
        .unwrap();

        // cascade=false + 仅存在交易 → 应成功，交易 category_id 设为 NULL
        delete_category(&conn, &cat_id, false).unwrap();
        assert!(find_category(&conn, &cat_id).unwrap().is_none());

        let tx_null_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE category_id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tx_null_count, 1);
    }
}
