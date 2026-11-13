//! Categories and their one-to-one P&L accounts.
//!
//! A category is the UI name for exactly one `income` / `expense` account; the
//! ledger truth lives on that account's postings. Before v2 the link was
//! implicit (`type + name`), so renaming a category silently orphaned its
//! account and the next quick-add created another one. `categories.account_id`
//! makes the link explicit and the unique index keeps it a pair.

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

impl CategoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CategoryType::Income => "income",
            CategoryType::Expense => "expense",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "income" => Some(CategoryType::Income),
            "expense" => Some(CategoryType::Expense),
            _ => None,
        }
    }
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
    /// The P&L account backing this category. `None` only for rows written
    /// before v2 whose backfill could not run — every normal path sets it.
    pub account_id: Option<String>,
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
const CATEGORY_COLUMNS: &str = "id, name, type, icon, account_id, created_at, updated_at";

/// Read a CategoryRecord from a row.
fn row_to_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<CategoryRecord> {
    let category_type =
        CategoryType::parse(&row.get::<_, String>(2)?).ok_or(rusqlite::Error::InvalidQuery)?;
    Ok(CategoryRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        category_type,
        icon: row.get(3)?,
        account_id: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

/// Find or create the P&L account that mirrors `(type, name)`.
///
/// An existing account is adopted only when no category claims it yet; that is
/// the legacy-orphan case, and adopting it keeps its history instead of
/// creating a same-named twin that would violate `idx_accounts_type_name`.
fn bind_account(
    conn: &Connection,
    name: &str,
    category_type: &CategoryType,
) -> Result<String, AppError> {
    let type_str = category_type.as_str();

    let existing: Option<String> = conn
        .query_row(
            "SELECT a.id
               FROM accounts a
              WHERE a.type = ?1 AND a.name = ?2
                AND NOT EXISTS (SELECT 1 FROM categories c WHERE c.account_id = a.id)",
            rusqlite::params![type_str, name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    conn.execute(
        "INSERT INTO accounts
             (id, name, type, currency, description, is_system, is_active,
              include_net_worth, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'CNY', '', 0, 1, 0, ?4, ?4)",
        rusqlite::params![&id, name, type_str, &now],
    )?;
    Ok(id)
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
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM categories WHERE type = ?1 ORDER BY name",
        CATEGORY_COLUMNS
    ))?;
    let categories = stmt
        .query_map([category_type.as_str()], row_to_category)?
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
    let row = conn
        .query_row(
            &format!(
                "SELECT {} FROM categories WHERE type = ?1 AND name = ?2",
                CATEGORY_COLUMNS
            ),
            rusqlite::params![category_type.as_str(), name],
            row_to_category,
        )
        .optional()?;
    Ok(row)
}

/// The P&L account a category posts to.
///
/// This replaces the old "look the account up by name" helper: the lookup is
/// now an explicit column read, so a renamed category can never spawn a second
/// account.
pub fn account_id_for_category(conn: &Connection, category_id: &str) -> Result<String, AppError> {
    conn.query_row(
        "SELECT account_id FROM categories WHERE id = ?1",
        [category_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()?
    .flatten()
    .ok_or_else(|| AppError::NotFound("errors.categoryNotBound".to_string()))
}

/// Match or create a category by name and type.
///
/// Lives in the db layer so the AI quick-add path can resolve/auto-create a
/// category inside the same transaction that writes the transaction itself.
/// Matching order: exact (case-insensitive, trimmed), then bidirectional
/// substring match, then auto-create.
pub fn resolve_or_create(
    conn: &Connection,
    name: &str,
    category_type: &CategoryType,
) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("分类名称不能为空".to_string()));
    }

    if let Some(cat) = find_by_type_and_name(conn, category_type, trimmed)? {
        return Ok(cat.id);
    }

    let lower = trimmed.to_lowercase();
    for cat in list_by_type(conn, category_type)? {
        let cat_lower = cat.name.to_lowercase();
        if cat_lower.contains(&lower) || lower.contains(&cat_lower) {
            return Ok(cat.id);
        }
    }

    create_category(conn, trimmed, category_type, None)
}

// ---------------------------------------------------------------------------
// Create operation
// ---------------------------------------------------------------------------

/// Create a category together with its mirror P&L account.
///
/// Both rows are written here rather than by the caller so there is no window
/// in which a category exists without an account. The caller is expected to
/// own the transaction (`commands::categories::category_create` does).
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

    if find_by_type_and_name(conn, category_type, name)?.is_some() {
        return Err(AppError::ValidationError(
            "errors.categoryNameDuplicate".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    let account_id = bind_account(conn, name, category_type)?;

    conn.execute(
        &format!(
            "INSERT INTO categories ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            CATEGORY_COLUMNS
        ),
        rusqlite::params![&id, name, category_type.as_str(), icon, &account_id, &now],
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

/// Update category fields, keeping the mirror account in sync.
///
/// The rename is applied to both rows: leaving the account behind is exactly
/// the drift this table's `account_id` column exists to prevent.
pub fn update_category(
    conn: &Connection,
    params: &UpdateCategoryParams<'_>,
) -> Result<(), AppError> {
    let current = find_category(conn, params.id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    let now = crate::utils::time::now_rfc3339();
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(raw_name) = params.name {
        let new_name = raw_name.trim();
        if new_name.is_empty() {
            return Err(AppError::ValidationError(
                "errors.categoryNameRequired".to_string(),
            ));
        }

        if let Some(existing) = find_by_type_and_name(conn, &current.category_type, new_name)? {
            if existing.id != params.id {
                return Err(AppError::ValidationError(
                    "errors.categoryNameDuplicate".to_string(),
                ));
            }
        }

        // Rename the account first: `idx_accounts_type_name` is UNIQUE, so a
        // collision must abort before the category row is touched.
        if let Some(account_id) = current.account_id.as_deref() {
            let taken: Option<String> = conn
                .query_row(
                    "SELECT id FROM accounts WHERE type = ?1 AND name = ?2 AND id <> ?3",
                    rusqlite::params![current.category_type.as_str(), new_name, account_id],
                    |row| row.get(0),
                )
                .optional()?;
            if taken.is_some() {
                return Err(AppError::ValidationError(
                    "errors.accountNameAlreadyExists".to_string(),
                ));
            }
            conn.execute(
                "UPDATE accounts SET name = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![new_name, &now, account_id],
            )?;
        }

        sets.push("name = ?".to_string());
        sql_params.push(Box::new(new_name.to_string()));
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
    let category = find_category(conn, id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    let budget_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM budgets WHERE category_id = ?1",
        [id],
        |row| row.get(0),
    )?;

    // Transactions reach a category either through the cached `category_id` or
    // through a posting on the mirror account (a split has only the latter).
    let transaction_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM (
             SELECT id FROM transactions WHERE category_id = ?1
             UNION
             SELECT p.transaction_id FROM postings p WHERE p.account_id = ?2
         )",
        rusqlite::params![id, category.account_id.as_deref()],
        |row| row.get(0),
    )?;

    Ok(DeletePreview {
        budget_count,
        transaction_count,
        // The mirror account carries ledger history: once it has postings the
        // category cannot be removed without destroying entries.
        can_delete: transaction_count == 0,
    })
}

/// Delete a category, its mirror account, and (when `cascade`) its budgets.
///
/// Refused while the account still has postings: the ledger, not the category
/// row, is the source of truth, so removal would silently drop history.
pub fn delete_category(conn: &Connection, id: &str, cascade: bool) -> Result<(), AppError> {
    let category = find_category(conn, id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    if let Some(account_id) = category.account_id.as_deref() {
        let postings: i64 = conn.query_row(
            "SELECT COUNT(*) FROM postings WHERE account_id = ?1",
            [account_id],
            |row| row.get(0),
        )?;
        if postings > 0 {
            return Err(AppError::ValidationError(
                "errors.categoryAccountInUse".to_string(),
            ));
        }
    }

    let budget_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM budgets WHERE category_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    if budget_count > 0 && !cascade {
        return Err(AppError::ValidationError(
            "errors.categoryHasBudgets".to_string(),
        ));
    }
    if cascade {
        conn.execute("DELETE FROM budgets WHERE category_id = ?1", [id])?;
    }

    // Defensive: the account has no postings (checked above), so any lingering
    // `category_id` reference is stale cache from a pre-v2 bug.
    conn.execute(
        "UPDATE transactions SET category_id = NULL WHERE category_id = ?1",
        [id],
    )?;

    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;

    if let Some(account_id) = category.account_id.as_deref() {
        conn.execute("DELETE FROM accounts WHERE id = ?1", [account_id])?;
    }

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
        crate::db::init_schema(&conn).unwrap();
        (dir, conn)
    }

    fn account_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap()
    }

    fn add_posting(conn: &Connection, account_id: &str) -> String {
        let now = crate::utils::time::now_rfc3339();
        let tx_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'test', ?2, ?2)",
            rusqlite::params![&tx_id, &now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES (?1, ?2, ?3, 100, 0, ?4)",
            rusqlite::params![Uuid::new_v4().to_string(), &tx_id, account_id, &now],
        )
        .unwrap();
        tx_id
    }

    #[test]
    fn test_create_category_creates_mirror_account() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, Some("🍔")).unwrap();

        let cat = find_category(&conn, &id).unwrap().unwrap();
        assert_eq!(cat.name, "餐饮");
        assert_eq!(cat.category_type, CategoryType::Expense);
        assert_eq!(cat.icon, Some("🍔".to_string()));

        let account_id = cat
            .account_id
            .expect("category must be bound to an account");
        assert_eq!(
            account_of(&conn, &account_id),
            ("餐饮".into(), "expense".into(), 0)
        );
    }

    fn account_of(conn: &Connection, id: &str) -> (String, String, i32) {
        conn.query_row(
            "SELECT name, type, include_net_worth FROM accounts WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
    }

    #[test]
    fn test_create_adopts_orphan_account_instead_of_duplicating() {
        let (_dir, conn) = test_conn();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system,
                                   is_active, include_net_worth, created_at, updated_at)
             VALUES ('orphan', '餐饮', 'expense', 'CNY', '', 0, 1, 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        assert_eq!(
            find_category(&conn, &id).unwrap().unwrap().account_id,
            Some("orphan".to_string())
        );
        assert_eq!(account_count(&conn), 1, "the orphan must be reused");
    }

    #[test]
    fn test_create_duplicate_name_same_type_blocked() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let result = create_category(&conn, "餐饮", &CategoryType::Expense, None);
        assert!(result.is_err());
        assert_eq!(account_count(&conn), 1);
    }

    #[test]
    fn test_create_same_name_different_type_allowed() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        assert!(create_category(&conn, "餐饮", &CategoryType::Income, None).is_ok());
    }

    #[test]
    fn test_list_by_type() {
        let (_dir, conn) = test_conn();
        create_category(&conn, "工资", &CategoryType::Income, None).unwrap();
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();

        let income = list_by_type(&conn, &CategoryType::Income).unwrap();
        assert_eq!(income.len(), 1);
        assert_eq!(income[0].name, "工资");

        let expense = list_by_type(&conn, &CategoryType::Expense).unwrap();
        assert_eq!(expense.len(), 1);
        assert_eq!(expense[0].name, "餐饮");
    }

    #[test]
    fn test_update_renames_mirror_account() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let account_id = find_category(&conn, &id)
            .unwrap()
            .unwrap()
            .account_id
            .unwrap();

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
        assert_eq!(account_of(&conn, &account_id).0, "外卖");
        assert_eq!(
            account_count(&conn),
            1,
            "rename must not create a second account"
        );
    }

    #[test]
    fn test_update_to_name_taken_by_unrelated_account_is_blocked() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system,
                                   is_active, include_net_worth, created_at, updated_at)
             VALUES ('other', '交通', 'expense', 'CNY', '', 0, 1, 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let err = update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: Some("交通"),
                icon: IconUpdateAction::NoChange,
            },
        )
        .unwrap_err();
        assert!(format!("{err}").contains("账户名称已存在"));
        // Nothing changed.
        assert_eq!(find_category(&conn, &id).unwrap().unwrap().name, "餐饮");
    }

    #[test]
    fn test_delete_category_removes_mirror_account() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        delete_category(&conn, &id, false).unwrap();

        assert!(find_category(&conn, &id).unwrap().is_none());
        assert_eq!(account_count(&conn), 0);
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
        assert!(result.is_err(), "invalid type must violate CHECK");
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

        // A transaction that no longer references the category but still posts
        // to its account (this is what a split looks like).
        let account_id = find_category(&conn, &cat_id)
            .unwrap()
            .unwrap()
            .account_id
            .unwrap();
        add_posting(&conn, &account_id);

        let preview = preview_delete(&conn, &cat_id).unwrap();
        assert_eq!(preview.budget_count, 1);
        assert_eq!(preview.transaction_count, 1);
        assert!(!preview.can_delete);

        // Refused before anything is deleted: the check runs first, so the
        // budget survives along with the category.
        let err = delete_category(&conn, &cat_id, true).unwrap_err();
        assert!(format!("{err}").contains("无法删除"));
        assert!(find_category(&conn, &cat_id).unwrap().is_some());
        let budgets: i64 = conn
            .query_row("SELECT COUNT(*) FROM budgets", [], |row| row.get(0))
            .unwrap();
        assert_eq!(budgets, 1);
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
        assert_eq!(cats[1].name, "Transport");
        assert_eq!(cats[2].category_type, CategoryType::Income);
        assert_eq!(cats[2].name, "Bonus");
        assert_eq!(cats[3].name, "Salary");
    }

    #[test]
    fn test_create_empty_name_blocked() {
        let (_dir, conn) = test_conn();
        assert!(create_category(&conn, "", &CategoryType::Expense, None).is_err());
        assert!(create_category(&conn, "   ", &CategoryType::Expense, None).is_err());
        assert_eq!(account_count(&conn), 0);
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
        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
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

        update_category(
            &conn,
            &UpdateCategoryParams {
                id: &id,
                name: Some("餐饮"),
                icon: IconUpdateAction::Set("🍔".to_string()),
            },
        )
        .unwrap();

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
        assert!(find_category(&conn, &id).unwrap().unwrap().icon.is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let (_dir, conn) = test_conn();
        assert!(delete_category(&conn, "nonexistent-id", false).is_err());
    }

    #[test]
    fn test_preview_delete_nonexistent() {
        let (_dir, conn) = test_conn();
        assert!(preview_delete(&conn, "nonexistent-id").is_err());
    }

    #[test]
    fn test_updated_at_updated_on_modify() {
        let (_dir, conn) = test_conn();
        let id = create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        let before = find_category(&conn, &id).unwrap().unwrap().updated_at;

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

        assert_ne!(
            find_category(&conn, &id).unwrap().unwrap().updated_at,
            before
        );
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

        match delete_category(&conn, &cat_id, false) {
            Err(AppError::ValidationError(msg)) => assert_eq!(msg, "errors.categoryHasBudgets"),
            other => panic!("expected categoryHasBudgets, got {other:?}"),
        }
        assert!(find_category(&conn, &cat_id).unwrap().is_some());
    }

    #[test]
    fn test_account_id_for_unbound_category() {
        let (_dir, conn) = test_conn();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO categories (id, name, type, created_at, updated_at)
             VALUES ('legacy', '老分类', 'expense', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let err = account_id_for_category(&conn, "legacy").unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));

        let missing = account_id_for_category(&conn, "nope").unwrap_err();
        assert!(matches!(missing, AppError::NotFound(_)));
    }

    #[test]
    fn test_resolve_or_create_is_idempotent() {
        let (_dir, conn) = test_conn();
        let first = resolve_or_create(&conn, "餐饮", &CategoryType::Expense).unwrap();
        let second = resolve_or_create(&conn, "餐饮", &CategoryType::Expense).unwrap();
        assert_eq!(first, second);
        assert_eq!(account_count(&conn), 1);
    }
}
