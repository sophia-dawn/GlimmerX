use chrono::Datelike;
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::categories::{find_category, CategoryType};
use crate::db::AppError;

// ---------------------------------------------------------------------------
// Type definitions
// ---------------------------------------------------------------------------

/// Budget period enum for budget cycle type.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BudgetPeriod {
    #[default]
    Monthly,
    Weekly,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Internal DB row representation for a budget.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetRecord {
    pub id: String,
    pub category_id: String,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Budget status with spending information for display.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetStatusRecord {
    pub id: String,
    pub category_id: String,
    pub category_name: String,
    pub category_icon: Option<String>,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: bool,
    /// Total spent in the current period (sum of expense postings)
    pub spent: i64,
    /// Remaining budget (amount - spent + rollover_amount)
    pub remaining: i64,
    /// True if spent > (amount + rollover_amount)
    pub over_budget: bool,
    /// Previous period's unused budget carried over (if rollover enabled)
    pub rollover_amount: i64,
    /// Budget available for spending (amount + rollover_amount)
    pub available: i64,
}

// ---------------------------------------------------------------------------
// Constants and helpers
// ---------------------------------------------------------------------------

/// Common columns for SELECT queries on budgets table.
const BUDGET_COLUMNS: &str = "id, category_id, amount, period, rollover, created_at, updated_at";

/// Read a BudgetRecord from a row.
fn row_to_budget(row: &rusqlite::Row<'_>) -> rusqlite::Result<BudgetRecord> {
    let period_str: String = row.get(3)?;
    let period = match period_str.as_str() {
        "monthly" => BudgetPeriod::Monthly,
        "weekly" => BudgetPeriod::Weekly,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(BudgetRecord {
        id: row.get(0)?,
        category_id: row.get(1)?,
        amount: row.get(2)?,
        period,
        rollover: row.get::<_, i32>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// List all budgets, sorted by creation date descending.
pub fn list_budgets(conn: &Connection) -> Result<Vec<BudgetRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM budgets ORDER BY created_at DESC",
        BUDGET_COLUMNS
    ))?;
    let budgets = stmt
        .query_map([], row_to_budget)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(budgets)
}

/// Find a single budget by ID.
pub fn find_budget(conn: &Connection, id: &str) -> Result<Option<BudgetRecord>, AppError> {
    let row = conn
        .query_row(
            &format!("SELECT {} FROM budgets WHERE id = ?1", BUDGET_COLUMNS),
            [id],
            row_to_budget,
        )
        .optional()?;
    Ok(row)
}

/// Find a budget by category ID (for uniqueness check).
pub fn find_by_category(
    conn: &Connection,
    category_id: &str,
) -> Result<Option<BudgetRecord>, AppError> {
    let row = conn
        .query_row(
            &format!(
                "SELECT {} FROM budgets WHERE category_id = ?1",
                BUDGET_COLUMNS
            ),
            [category_id],
            row_to_budget,
        )
        .optional()?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// Budget status calculation
// ---------------------------------------------------------------------------

/// Calculate budget status for a single budget.
/// Computes spent amount from transactions with matching category_id in the date range.
#[allow(dead_code)]
pub fn calculate_budget_status(
    conn: &Connection,
    budget: &BudgetRecord,
    year: i32,
    month: i32,
) -> Result<BudgetStatusRecord, AppError> {
    let category = find_category(conn, &budget.category_id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    let from_date = format!("{:04}-{:02}-01", year, month);
    let last_day = if month == 12 {
        31
    } else {
        let next_month = chrono::NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap());
        (next_month - chrono::Duration::days(1)).day()
    };
    let to_date = format!("{:04}-{:02}-{:02}", year, month, last_day);

    let spent: i64 = conn.query_row(
        r#"
        SELECT COALESCE(SUM(p.amount), 0)
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id
        JOIN accounts a ON a.id = p.account_id
        WHERE t.category_id = ?1
          AND a.type = 'expense'
          AND t.date >= ?2
          AND t.date <= ?3
          AND t.deleted_at IS NULL
        "#,
        rusqlite::params![&budget.category_id, &from_date, &to_date],
        |row| row.get(0),
    )?;

    // TODO: Implement rollover calculation in P1 (BUD-4)
    // For now, rollover_amount is always 0
    let rollover_amount: i64 = 0;

    let available = budget.amount + rollover_amount;
    let remaining = available - spent;
    let over_budget = spent > available;

    Ok(BudgetStatusRecord {
        id: budget.id.clone(),
        category_id: budget.category_id.clone(),
        category_name: category.name,
        category_icon: category.icon,
        amount: budget.amount,
        period: budget.period.clone(),
        rollover: budget.rollover,
        spent,
        remaining,
        over_budget,
        rollover_amount,
        available,
    })
}

/// List all budget statuses with spending information for a specific year/month.
/// Uses a single aggregated query to avoid N+1 problem.
pub fn list_budget_statuses(
    conn: &Connection,
    year: i32,
    month: i32,
) -> Result<Vec<BudgetStatusRecord>, AppError> {
    let from_date = format!("{:04}-{:02}-01", year, month);
    let last_day = if month == 12 {
        31
    } else {
        let next_month = chrono::NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap());
        (next_month - chrono::Duration::days(1)).day()
    };
    let to_date = format!("{:04}-{:02}-{:02}", year, month, last_day);

    let mut stmt = conn
        .prepare(
            r#"
        SELECT 
            b.id, b.category_id, c.name, c.icon, b.amount, b.period, b.rollover,
            COALESCE(SUM(CASE WHEN a.type = 'expense' THEN p.amount ELSE 0 END), 0) as spent
        FROM budgets b
        JOIN categories c ON c.id = b.category_id
        LEFT JOIN transactions t ON t.category_id = b.category_id 
            AND t.date >= ?1 AND t.date <= ?2 AND t.deleted_at IS NULL
        LEFT JOIN postings p ON p.transaction_id = t.id 
        LEFT JOIN accounts a ON a.id = p.account_id
        GROUP BY b.id
        ORDER BY b.created_at DESC
        "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let statuses = stmt
        .query_map(rusqlite::params![&from_date, &to_date], |row| {
            let period_str: String = row.get(5)?;
            let period = match period_str.as_str() {
                "monthly" => BudgetPeriod::Monthly,
                "weekly" => BudgetPeriod::Weekly,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let rollover: bool = row.get::<_, i32>(6)? != 0;
            let amount: i64 = row.get(4)?;
            let spent: i64 = row.get(7)?;
            let rollover_amount: i64 = 0;
            let available = amount + rollover_amount;
            let remaining = available - spent;
            let over_budget = spent > available;

            Ok(BudgetStatusRecord {
                id: row.get(0)?,
                category_id: row.get(1)?,
                category_name: row.get(2)?,
                category_icon: row.get(3)?,
                amount,
                period,
                rollover,
                spent,
                remaining,
                over_budget,
                rollover_amount,
                available,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(statuses)
}

// ---------------------------------------------------------------------------
// Create operation
// ---------------------------------------------------------------------------

/// Create a new budget for an expense category.
/// Validates:
/// - Category must exist and be expense type
/// - No duplicate budget for same category
/// - Amount must be positive
pub fn create_budget(
    conn: &Connection,
    category_id: &str,
    amount: i64,
    period: &BudgetPeriod,
    rollover: bool,
) -> Result<String, AppError> {
    // Validate category exists and is expense type
    let category = find_category(conn, category_id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    if category.category_type != CategoryType::Expense {
        return Err(AppError::ValidationError(
            "errors.budgetOnlyForExpense".to_string(),
        ));
    }

    // Check for duplicate budget
    if let Some(_existing) = find_by_category(conn, category_id)? {
        return Err(AppError::ValidationError(
            "errors.budgetAlreadyExists".to_string(),
        ));
    }

    // Validate amount is positive
    if amount <= 0 {
        return Err(AppError::ValidationError(
            "errors.budgetAmountPositive".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    let period_str = match period {
        BudgetPeriod::Monthly => "monthly",
        BudgetPeriod::Weekly => "weekly",
    };
    let rollover_int = if rollover { 1 } else { 0 };

    conn.execute(
        &format!(
            "INSERT INTO budgets ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            BUDGET_COLUMNS
        ),
        rusqlite::params![&id, category_id, amount, period_str, rollover_int, &now],
    )?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Update operation
// ---------------------------------------------------------------------------

/// Parameters for updating a budget.
pub struct UpdateBudgetParams<'a> {
    pub id: &'a str,
    pub amount: Option<i64>,
    pub period: Option<BudgetPeriod>,
    pub rollover: Option<bool>,
}

/// Update budget fields.
pub fn update_budget(conn: &Connection, params: &UpdateBudgetParams<'_>) -> Result<(), AppError> {
    // Verify budget exists
    let _budget = find_budget(conn, params.id)?
        .ok_or_else(|| AppError::ValidationError("errors.budgetNotFound".to_string()))?;

    // Validate amount if provided
    if let Some(amount) = params.amount {
        if amount <= 0 {
            return Err(AppError::ValidationError(
                "errors.budgetAmountPositive".to_string(),
            ));
        }
    }

    let now = crate::utils::time::now_rfc3339();
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(val) = params.amount {
        sets.push("amount = ?".to_string());
        sql_params.push(Box::new(val));
    }

    if let Some(ref val) = params.period {
        sets.push("period = ?".to_string());
        let period_str = match val {
            BudgetPeriod::Monthly => "monthly",
            BudgetPeriod::Weekly => "weekly",
        };
        sql_params.push(Box::new(period_str.to_string()));
    }

    if let Some(val) = params.rollover {
        sets.push("rollover = ?".to_string());
        let rollover_int = if val { 1 } else { 0 };
        sql_params.push(Box::new(rollover_int));
    }

    if sets.is_empty() {
        return Ok(());
    }

    sets.push("updated_at = ?".to_string());
    sql_params.push(Box::new(now));
    sql_params.push(Box::new(params.id.to_string()));

    let sql = format!("UPDATE budgets SET {} WHERE id = ?", sets.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();
    stmt.execute(rusqlite::params_from_iter(param_refs))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete operation
// ---------------------------------------------------------------------------

/// Delete a budget by ID.
pub fn delete_budget(conn: &Connection, id: &str) -> Result<(), AppError> {
    // Verify budget exists
    let _budget = find_budget(conn, id)?
        .ok_or_else(|| AppError::ValidationError("errors.budgetNotFound".to_string()))?;

    conn.execute("DELETE FROM budgets WHERE id = ?1", [id])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::categories::{create_category, CategoryType};
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

            CREATE TABLE budgets (
                id          TEXT PRIMARY KEY,
                category_id TEXT NOT NULL REFERENCES categories(id),
                amount      INTEGER NOT NULL,
                period      TEXT NOT NULL,
                rollover    INTEGER NOT NULL DEFAULT 0,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE accounts (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                type                TEXT NOT NULL CHECK (type IN ('asset', 'liability', 'income', 'expense', 'equity')),
                currency            TEXT NOT NULL DEFAULT 'CNY',
                description         TEXT NOT NULL DEFAULT '',
                account_number      TEXT,
                is_system           INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
                iban                TEXT,
                is_active           INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
                include_net_worth   INTEGER NOT NULL DEFAULT 1 CHECK (include_net_worth IN (0, 1)),
                created_at          TEXT NOT NULL,
                updated_at          TEXT NOT NULL
            );

            CREATE TABLE transactions (
                id          TEXT PRIMARY KEY,
                date        TEXT NOT NULL,
                description TEXT NOT NULL,
                category_id TEXT REFERENCES categories(id),
                is_reconciled INTEGER NOT NULL DEFAULT 0 CHECK (is_reconciled IN (0, 1)),
                deleted_at  TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE postings (
                id             TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
                account_id     TEXT NOT NULL REFERENCES accounts(id),
                amount         INTEGER NOT NULL,
                sequence       INTEGER NOT NULL DEFAULT 0,
                created_at     TEXT NOT NULL
            );

            CREATE UNIQUE INDEX idx_categories_type_name ON categories(type, name);
            CREATE INDEX idx_categories_type ON categories(type);
            CREATE UNIQUE INDEX idx_accounts_type_name ON accounts(type, name);
            CREATE INDEX idx_postings_transaction ON postings(transaction_id);
            CREATE INDEX idx_postings_account ON postings(account_id);
            CREATE INDEX idx_transactions_date ON transactions(date);
            "#,
        )
        .unwrap();
        (dir, conn)
    }

    fn make_category(conn: &Connection, name: &str, category_type: &CategoryType) -> String {
        create_category(conn, name, category_type, None).unwrap()
    }

    fn make_account(conn: &Connection, name: &str, account_type: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'CNY', '', 0, ?4, ?4)",
            rusqlite::params![&id, name, account_type, &now],
        )
        .unwrap();
        id
    }

    #[allow(dead_code)]
    fn make_transaction(
        conn: &Connection,
        date: &str,
        description: &str,
        postings: &[(String, i64)],
    ) -> String {
        let tx_id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, is_reconciled, deleted_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, NULL, ?4, ?4)",
            rusqlite::params![&tx_id, date, description, &now],
        )
        .unwrap();

        for (seq, (account_id, amount)) in postings.iter().enumerate() {
            let posting_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![&posting_id, &tx_id, account_id, *amount, seq as i32, &now],
            )
            .unwrap();
        }

        tx_id
    }

    fn make_transaction_with_category(
        conn: &Connection,
        date: &str,
        description: &str,
        category_id: &str,
        postings: &[(String, i64)],
    ) -> String {
        let tx_id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, is_reconciled, deleted_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, NULL, ?5, ?5)",
            rusqlite::params![&tx_id, date, description, category_id, &now],
        )
        .unwrap();

        for (seq, (account_id, amount)) in postings.iter().enumerate() {
            let posting_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![&posting_id, &tx_id, account_id, *amount, seq as i32, &now],
            )
            .unwrap();
        }

        tx_id
    }

    #[test]
    fn test_create_budget() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        assert!(!budget_id.is_empty());

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.category_id, cat_id);
        assert_eq!(budget.amount, 10000);
        assert_eq!(budget.period, BudgetPeriod::Monthly);
        assert!(!budget.rollover);
    }

    #[test]
    fn test_create_budget_income_category_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "工资", &CategoryType::Income);

        let result = create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetOnlyForExpense");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_create_duplicate_budget_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);

        create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let result = create_budget(&conn, &cat_id, 5000, &BudgetPeriod::Weekly, true);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetAlreadyExists");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_create_budget_zero_amount_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);

        let result = create_budget(&conn, &cat_id, 0, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetAmountPositive");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }

        let result = create_budget(&conn, &cat_id, -100, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_budget() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: Some(15000),
                period: Some(BudgetPeriod::Weekly),
                rollover: Some(true),
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.amount, 15000);
        assert_eq!(budget.period, BudgetPeriod::Weekly);
        assert!(budget.rollover);
    }

    #[test]
    fn test_update_budget_zero_amount_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let result = update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: Some(0),
                period: None,
                rollover: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_budget() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        delete_budget(&conn, &budget_id).unwrap();
        assert!(find_budget(&conn, &budget_id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_budget() {
        let (_dir, conn) = test_conn();

        let result = delete_budget(&conn, "nonexistent-id");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetNotFound");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_list_budgets() {
        let (_dir, conn) = test_conn();
        let cat1_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let cat2_id = make_category(&conn, "交通", &CategoryType::Expense);

        create_budget(&conn, &cat1_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        create_budget(&conn, &cat2_id, 5000, &BudgetPeriod::Weekly, true).unwrap();

        let budgets = list_budgets(&conn).unwrap();
        assert_eq!(budgets.len(), 2);
    }

    #[test]
    fn test_find_by_category() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let found = find_by_category(&conn, &cat_id).unwrap().unwrap();
        assert_eq!(found.id, budget_id);

        // Category without budget should return None
        let cat2_id = make_category(&conn, "交通", &CategoryType::Expense);
        assert!(find_by_category(&conn, &cat2_id).unwrap().is_none());
    }

    #[test]
    fn test_budget_status_no_spending() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let _expense_acct = make_account(&conn, "餐饮", "expense");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 0);
        assert_eq!(status.remaining, 10000);
        assert!(!status.over_budget);
    }

    #[test]
    fn test_budget_status_with_spending() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let expense_acct = make_account(&conn, "餐饮", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        make_transaction_with_category(
            &conn,
            "2024-04-15",
            "午餐",
            &cat_id,
            &[(expense_acct.clone(), 5000), (asset_acct.clone(), -5000)],
        );

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 5000);
        assert_eq!(status.remaining, 5000);
        assert!(!status.over_budget);
    }

    #[test]
    fn test_budget_status_ignores_other_dates() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let expense_acct = make_account(&conn, "餐饮", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        make_transaction_with_category(
            &conn,
            "2024-01-15",
            "历史消费",
            &cat_id,
            &[(expense_acct.clone(), 5000), (asset_acct.clone(), -5000)],
        );

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 0);
    }

    #[test]
    fn test_budget_status_ignores_deleted_transactions() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let expense_acct = make_account(&conn, "餐饮", "expense");
        let _asset_acct = make_account(&conn, "Cash", "asset");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        let tx_id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, is_reconciled, deleted_at, created_at, updated_at)
             VALUES (?1, '2024-04-15', ?2, ?3, 0, ?4, ?4, ?4)",
            rusqlite::params![&tx_id, "已删除", &cat_id, &now],
        )
        .unwrap();

        let posting_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            rusqlite::params![&posting_id, &tx_id, &expense_acct, 5000, &now],
        )
        .unwrap();

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 0);
    }

    #[test]
    fn test_list_budget_statuses() {
        let (_dir, conn) = test_conn();
        let cat1_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let cat2_id = make_category(&conn, "交通", &CategoryType::Expense);
        make_account(&conn, "餐饮", "expense");
        make_account(&conn, "交通", "expense");

        create_budget(&conn, &cat1_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        create_budget(&conn, &cat2_id, 5000, &BudgetPeriod::Weekly, true).unwrap();

        let statuses = list_budget_statuses(&conn, 2024, 4).unwrap();
        assert_eq!(statuses.len(), 2);

        let food_status = statuses.iter().find(|s| s.category_name == "餐饮").unwrap();
        assert!(food_status.category_icon.is_none());

        let transport_status = statuses.iter().find(|s| s.category_name == "交通").unwrap();
        assert!(transport_status.rollover);
    }

    #[test]
    fn test_create_budget_with_weekly_period() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "交通", &CategoryType::Expense);
        let budget_id = create_budget(&conn, &cat_id, 5000, &BudgetPeriod::Weekly, true).unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.period, BudgetPeriod::Weekly);
        assert!(budget.rollover);
    }

    #[test]
    fn test_update_budget_partial() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        // Update only amount
        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: Some(15000),
                period: None,
                rollover: None,
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.amount, 15000);
        assert_eq!(budget.period, BudgetPeriod::Monthly); // unchanged
        assert!(!budget.rollover); // unchanged

        // Update only rollover
        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: None,
                period: None,
                rollover: Some(true),
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert!(budget.rollover);
    }

    #[test]
    fn test_update_budget_no_changes() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        // No changes should succeed without error
        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: None,
                period: None,
                rollover: None,
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.amount, 10000);
    }

    #[test]
    fn test_update_nonexistent_budget() {
        let (_dir, conn) = test_conn();

        let result = update_budget(
            &conn,
            &UpdateBudgetParams {
                id: "nonexistent-id",
                amount: Some(10000),
                period: None,
                rollover: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_category_icon_included() {
        let (_dir, conn) = test_conn();
        let cat_id = crate::db::categories::create_category(
            &conn,
            "餐饮",
            &CategoryType::Expense,
            Some("🍔"),
        )
        .unwrap();
        make_account(&conn, "餐饮", "expense");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.category_icon, Some("🍔".to_string()));
    }

    #[test]
    fn test_budget_status_different_category_same_account_name() {
        let (_dir, conn) = test_conn();
        let cat1_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let cat2_id = make_category(&conn, "外卖", &CategoryType::Expense);

        let food_acct = make_account(&conn, "餐饮", "expense");
        let delivery_acct = make_account(&conn, "外卖", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        create_budget(&conn, &cat1_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        create_budget(&conn, &cat2_id, 5000, &BudgetPeriod::Monthly, false).unwrap();

        make_transaction_with_category(
            &conn,
            "2024-04-15",
            "午餐",
            &cat1_id,
            &[(food_acct.clone(), 3000), (asset_acct.clone(), -3000)],
        );

        make_transaction_with_category(
            &conn,
            "2024-04-15",
            "外卖",
            &cat2_id,
            &[(delivery_acct.clone(), 2000), (asset_acct.clone(), -2000)],
        );

        let statuses = list_budget_statuses(&conn, 2024, 4).unwrap();

        let food_status = statuses.iter().find(|s| s.category_name == "餐饮").unwrap();
        assert_eq!(food_status.spent, 3000);

        let delivery_status = statuses.iter().find(|s| s.category_name == "外卖").unwrap();
        assert_eq!(delivery_status.spent, 2000);
    }
}
